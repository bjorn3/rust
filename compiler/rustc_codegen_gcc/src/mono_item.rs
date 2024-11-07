#[cfg(feature = "master")]
use gccjit::{FnAttribute, VarAttribute};
use rustc_codegen_ssa::traits::PreDefineCodegenMethods;
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrFlags;
use rustc_middle::mir::mono::{Linkage, Visibility};
use rustc_middle::ty::layout::FnAbiOf;
use rustc_middle::ty::{self, Instance, TypeVisitableExt};

use crate::context::CodegenCx;
use crate::{attributes, base};

impl<'gcc, 'tcx> PreDefineCodegenMethods<'tcx> for CodegenCx<'gcc, 'tcx> {
    #[cfg_attr(not(feature = "master"), allow(unused_variables))]
    fn predefine_static(
        &mut self,
        def_id: DefId,
        _linkage: Linkage,
        visibility: Visibility,
        symbol_name: &str,
    ) {
        let attrs = self.tcx.codegen_fn_attrs(def_id);
        let instance = Instance::mono(self.tcx, def_id);
        // The lowering the rust type of the static to GCC rarely agrees with
        // what `codegen_static` would do, so let the latter figure out the GCC
        // type from the actual evaluated initializer.
        let gcc_type = self.type_struct(&[], false);

        let is_tls = attrs.flags.contains(CodegenFnAttrFlags::THREAD_LOCAL);
        let global = self.define_global(symbol_name, gcc_type, is_tls, attrs.link_section);
        #[cfg(feature = "master")]
        global.add_attribute(VarAttribute::Visibility(base::visibility_to_gcc(visibility)));

        // TODO(antoyo): set linkage.
        self.instances.borrow_mut().insert(instance, global);
    }

    #[cfg_attr(not(feature = "master"), allow(unused_variables))]
    fn predefine_fn(
        &mut self,
        instance: Instance<'tcx>,
        linkage: Linkage,
        visibility: Visibility,
        symbol_name: &str,
    ) {
        assert!(!instance.args.has_infer());

        let fn_abi = self.fn_abi_of_instance(instance, ty::List::empty());
        self.linkage.set(base::linkage_to_gcc(linkage));
        let decl = self.declare_fn(symbol_name, fn_abi);
        //let attrs = self.tcx.codegen_instance_attrs(instance.def);

        attributes::from_fn_attrs(self, decl, instance);

        // If we're compiling the compiler-builtins crate, e.g., the equivalent of
        // compiler-rt, then we want to implicitly compile everything with hidden
        // visibility as we're going to link this object all over the place but
        // don't want the symbols to get exported.
        if linkage != Linkage::Internal && self.tcx.is_compiler_builtins(LOCAL_CRATE) {
            #[cfg(feature = "master")]
            decl.add_attribute(FnAttribute::Visibility(gccjit::Visibility::Hidden));
        } else if visibility != Visibility::Default {
            #[cfg(feature = "master")]
            decl.add_attribute(FnAttribute::Visibility(base::visibility_to_gcc(visibility)));
        }

        // TODO(antoyo): call set_link_section() to allow initializing argc/argv.
        // TODO(antoyo): set unique comdat.
        // TODO(antoyo): use inline attribute from there in linkage.set() above.

        self.functions.borrow_mut().insert(symbol_name.to_string(), decl);
        self.function_instances.borrow_mut().insert(instance, decl);
    }
}
