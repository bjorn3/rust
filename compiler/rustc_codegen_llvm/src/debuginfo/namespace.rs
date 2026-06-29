// Namespace Handling.

use rustc_middle::ty::{self, Instance};

use crate::common::CodegenCx;

pub(crate) fn mangled_name_of_instance<'a, 'tcx>(
    cx: &CodegenCx<'a, 'tcx>,
    instance: Instance<'tcx>,
) -> ty::SymbolName<'tcx> {
    cx.tcx.symbol_name(instance)
}
