//! Allocator shim
// Adapted from rustc

use rustc_ast::expand::allocator::{AllocatorMethod, AllocatorTy, default_fn_name, global_fn_name};
use rustc_symbol_mangling::mangle_internal_symbol;

use crate::prelude::*;

pub(crate) fn codegen(tcx: TyCtxt<'_>, module: &mut dyn Module, methods: &[AllocatorMethod]) {
    let usize_ty = module.target_config().pointer_type();

    for method in methods {
        let mut arg_tys = Vec::with_capacity(method.inputs.len());
        for input in method.inputs.iter() {
            match input.ty {
                AllocatorTy::Layout => {
                    arg_tys.push(usize_ty); // size
                    arg_tys.push(usize_ty); // align
                }
                AllocatorTy::Ptr => arg_tys.push(usize_ty),
                AllocatorTy::Usize => arg_tys.push(usize_ty),

                AllocatorTy::Never | AllocatorTy::ResultPtr | AllocatorTy::Unit => {
                    panic!("invalid allocator arg")
                }
            }
        }
        let output = match method.output {
            AllocatorTy::ResultPtr => Some(usize_ty),
            AllocatorTy::Never | AllocatorTy::Unit => None,

            AllocatorTy::Layout | AllocatorTy::Usize | AllocatorTy::Ptr => {
                panic!("invalid allocator output")
            }
        };

        let sig = Signature {
            call_conv: module.target_config().default_call_conv,
            params: arg_tys.iter().cloned().map(AbiParam::new).collect(),
            returns: output.into_iter().map(AbiParam::new).collect(),
        };
        crate::common::create_wrapper_function(
            module,
            sig,
            &mangle_internal_symbol(tcx, &global_fn_name(method.name)),
            &mangle_internal_symbol(tcx, &default_fn_name(method.name)),
        );
    }
}
