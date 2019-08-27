use syntax::ast::Mutability;

use rustc::ty::{self, ParamEnv, Ty, TyCtxt, Instance, TypeFoldable};
use rustc::ty::layout::{Size, Align, HasDataLayout};
use rustc::mir::interpret::{Scalar, Pointer, InterpResult, PointerArithmetic};

use super::{Allocation, InterpCx, Machine};

/// Creates a dynamic vtable for the given type and vtable origin. This is used only for
/// objects.
///
/// The `trait_ref` encodes the erased self type. Hence if we are
/// making an object `Foo<Trait>` from a value of type `Foo<T>`, then
/// `trait_ref` would map `T:Trait`.
pub fn get_vtable<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    poly_trait_ref: Option<ty::PolyExistentialTraitRef<'tcx>>,
) -> InterpResult<'tcx, Pointer<()>> {
    trace!("get_vtable(trait_ref={:?})", poly_trait_ref);

    let (ty, poly_trait_ref) = tcx.erase_regions(&(ty, poly_trait_ref));

    // All vtables must be monomorphic, bail out otherwise.
    if ty.needs_subst() || poly_trait_ref.needs_subst() {
        throw_inval!(TooGeneric);
    }

    let alloc_id = match tcx.alloc_map.lock().reserve_vtable(ty, poly_trait_ref) {
        Err(alloc_id) => {
            // This means we guarantee that there are no duplicate vtables, we will
            // always use the same vtable for the same (Type, Trait) combination.
            // That's not what happens in rustc, but emulating per-crate deduplication
            // does not sound like it actually makes anything any better.
            let vtable = Pointer::new(alloc_id, Size::from_bytes(0));
            return Ok(vtable);
        }
        Ok(alloc_id) => alloc_id,
    };

    let methods = if let Some(poly_trait_ref) = poly_trait_ref {
        let trait_ref = poly_trait_ref.with_self_ty(tcx, ty);
        let trait_ref = tcx.erase_regions(&trait_ref);

        tcx.vtable_methods(trait_ref)
    } else {
        &[]
    };

    let layout = tcx
        .layout_of(ParamEnv::reveal_all().and(ty)) // FIXME is this param env correct?
        .map_err(|layout| err_inval!(Layout(layout)))?;
    assert!(!layout.is_unsized(), "can't create a vtable for an unsized type");
    let size = layout.size.bytes();
    let align = layout.align.abi.bytes();

    let ptr_size = tcx.pointer_size();
    let ptr_align = tcx.data_layout.pointer_align.abi;
    // /////////////////////////////////////////////////////////////////////////////////////////
    // If you touch this code, be sure to also make the corresponding changes to
    // `get_vtable` in rust_codegen_llvm/meth.rs
    // /////////////////////////////////////////////////////////////////////////////////////////
    let mut vtable_alloc = Allocation::undef(
        ptr_size * (3 + methods.len() as u64),
        ptr_align,
    );
    let vtable = Pointer::from(alloc_id);

    let drop = Instance::resolve_drop_in_place(tcx, ty);
    let drop = tcx.alloc_map.lock().create_fn_alloc(drop);

    // no need to do any alignment checks on the memory accesses below, because we know the
    // allocation is correctly aligned as we created it above. Also we're only offsetting by
    // multiples of `ptr_align`, which means that it will stay aligned to `ptr_align`.
    vtable_alloc.write_ptr_sized(&tcx, vtable, Scalar::Ptr(drop.into()).into())?;

    let size_ptr = vtable.offset(ptr_size, &tcx)?;
    vtable_alloc.write_ptr_sized(&tcx, size_ptr, Scalar::from_uint(size, ptr_size).into())?;
    let align_ptr = vtable.offset(ptr_size * 2, &tcx)?;
    vtable_alloc.write_ptr_sized(&tcx, align_ptr, Scalar::from_uint(align, ptr_size).into())?;

    for (i, method) in methods.iter().enumerate() {
        if let Some((def_id, substs)) = *method {
            // resolve for vtable: insert shims where needed
            let instance = ty::Instance::resolve_for_vtable(
                tcx,
                ParamEnv::reveal_all(), // FIXME is this correct?
                def_id,
                substs,
            ).ok_or_else(|| err_inval!(TooGeneric))?;
            let fn_ptr = tcx.alloc_map.lock().create_fn_alloc(instance);
            let method_ptr = vtable.offset(ptr_size * (3 + i as u64), &tcx)?;
            vtable_alloc.write_ptr_sized(&tcx, method_ptr, Scalar::Ptr(fn_ptr.into()).into())?;
        }
    }

    vtable_alloc.mutability = Mutability::Immutable;
    tcx.alloc_map.lock().set_alloc_id_memory(vtable.alloc_id, tcx.intern_const_alloc(vtable_alloc));

    Ok(vtable)
}

impl<'mir, 'tcx, M: Machine<'mir, 'tcx>> InterpCx<'mir, 'tcx, M> {
    /// Returns the drop fn instance as well as the actual dynamic type
    pub fn read_drop_type_from_vtable(
        &self,
        vtable: Scalar<M::PointerTag>,
    ) -> InterpResult<'tcx, (ty::Instance<'tcx>, Ty<'tcx>)> {
        // we don't care about the pointee type, we just want a pointer
        let vtable = self.memory.check_ptr_access(
            vtable,
            self.tcx.data_layout.pointer_size,
            self.tcx.data_layout.pointer_align.abi,
        )?.expect("cannot be a ZST");
        let drop_fn = self.memory
            .get(vtable.alloc_id)?
            .read_ptr_sized(self, vtable)?
            .not_undef()?;
        // We *need* an instance here, no other kind of function value, to be able
        // to determine the type.
        let drop_instance = self.memory.get_fn(drop_fn)?.as_instance()?;
        trace!("Found drop fn: {:?}", drop_instance);
        let fn_sig = drop_instance.ty(*self.tcx).fn_sig(*self.tcx);
        let fn_sig = self.tcx.normalize_erasing_late_bound_regions(self.param_env, &fn_sig);
        // The drop function takes `*mut T` where `T` is the type being dropped, so get that.
        let ty = fn_sig.inputs()[0].builtin_deref(true).unwrap().ty;
        Ok((drop_instance, ty))
    }

    pub fn read_size_and_align_from_vtable(
        &self,
        vtable: Scalar<M::PointerTag>,
    ) -> InterpResult<'tcx, (Size, Align)> {
        let pointer_size = self.pointer_size();
        // We check for size = 3*ptr_size, that covers the drop fn (unused here),
        // the size, and the align (which we read below).
        let vtable = self.memory.check_ptr_access(
            vtable,
            3*pointer_size,
            self.tcx.data_layout.pointer_align.abi,
        )?.expect("cannot be a ZST");
        let alloc = self.memory.get(vtable.alloc_id)?;
        let size = alloc.read_ptr_sized(
            self,
            vtable.offset(pointer_size, self)?
        )?.not_undef()?;
        let size = self.force_bits(size, pointer_size)? as u64;
        let align = alloc.read_ptr_sized(
            self,
            vtable.offset(pointer_size * 2, self)?,
        )?.not_undef()?;
        let align = self.force_bits(align, pointer_size)? as u64;

        if size >= self.tcx.data_layout().obj_size_bound() {
            throw_ub_format!("invalid vtable: \
                size is bigger than largest supported object");
        }
        Ok((Size::from_bytes(size), Align::from_bytes(align).unwrap()))
    }
}
