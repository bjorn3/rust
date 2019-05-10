use crate::back::lto::{LtoModuleCodegen, SerializedModule, ThinModule};
use crate::back::write::{CodegenContext, ModuleConfig, FatLTOInput};
use crate::{CompiledModule, ModuleCodegen};

use rustc::dep_graph::WorkProduct;
use rustc_errors::{FatalError, Handler};

pub trait WriteBackendMethods: 'static + Sized + Clone {
    type Context: ?Sized;
    type TargetMachine;

    type Module: Send + Sync;
    type ModuleBuffer: ModuleBufferMethods;

    type ThinData: Send + Sync;
    type ThinBuffer: ThinBufferMethods;

    /// Optimize this module in isolation
    unsafe fn optimize(
        cgcx: &CodegenContext<Self>,
        diag_handler: &Handler,
        module: &mut ModuleCodegen<Self::Module>,
        config: &ModuleConfig,
    ) -> Result<(), FatalError>;

    /// Emit an object file
    unsafe fn codegen(
        cgcx: &CodegenContext<Self>,
        diag_handler: &Handler,
        module: ModuleCodegen<Self::Module>,
        config: &ModuleConfig,
    ) -> Result<CompiledModule, FatalError>;

    // ==== single module operations for LTO ====

    fn prepare_thin(
        module: ModuleCodegen<Self::Module>
    ) -> (String, Self::ThinBuffer);

    fn serialize_module(
        module: ModuleCodegen<Self::Module>
    ) -> (String, Self::ModuleBuffer);

    // ==== cross module operations for LTO ====

    /// Performs fat LTO by merging all modules into a single one and returning it
    /// for further optimization.
    fn run_fat_lto(
        cgcx: &CodegenContext<Self>,
        modules: Vec<FatLTOInput<Self>>,
        cached_modules: Vec<(SerializedModule<Self::ModuleBuffer>, WorkProduct)>,
    ) -> Result<LtoModuleCodegen<Self>, FatalError>;

    /// Performs thin LTO by performing necessary global analysis and returning two
    /// lists, one of the modules that need optimization and another for modules that
    /// can simply be copied over from the incr. comp. cache.
    fn run_thin_lto(
        cgcx: &CodegenContext<Self>,
        modules: Vec<(String, Self::ThinBuffer)>,
        cached_modules: Vec<(SerializedModule<Self::ModuleBuffer>, WorkProduct)>,
    ) -> Result<(Vec<LtoModuleCodegen<Self>>, Vec<WorkProduct>), FatalError>;

    fn run_lto_pass_manager(
        cgcx: &CodegenContext<Self>,
        llmod: &mut ModuleCodegen<Self::Module>,
        config: &ModuleConfig,
    ) ;

    unsafe fn optimize_thin(
        cgcx: &CodegenContext<Self>,
        thin: &mut ThinModule<Self>,
    ) -> Result<ModuleCodegen<Self::Module>, FatalError>;

    fn print_pass_timings(&self);
}

pub trait ThinBufferMethods: Send + Sync {
    fn data(&self) -> &[u8];
}

pub trait ModuleBufferMethods: Send + Sync {
    fn data(&self) -> &[u8];
}
