use rustc_hir::def_id::DefId;
use rustc_middle::mir::interpret::ConstAllocation;

use super::BackendTypes;

pub trait StaticCodegenMethods: BackendTypes {
    fn make_vtable(&self, alloc: ConstAllocation<'_>) -> Self::Vtable;
    fn codegen_static(&mut self, def_id: DefId);
}

pub trait StaticBuilderMethods: BackendTypes {
    fn get_static(&mut self, def_id: DefId) -> Self::Value;
    fn get_vtable_addr(&mut self, s: Self::Vtable) -> Self::Value;
    fn get_anon_static_addr(&self, alloc: ConstAllocation<'_>) -> Self::Value;
}
