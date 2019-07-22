//! The Rust compiler.
//!
//! # Note
//!
//! This API is completely unstable and subject to change.

#![doc(html_root_url = "https://doc.rust-lang.org/nightly/")]
#![deny(warnings)]

#![feature(box_syntax)]

use std::any::Any;
use std::sync::mpsc;
use std::fs::File;
use std::path::Path;

use syntax::symbol::Symbol;
use rustc::session::{Session, config::CrateType};
use rustc::util::common::ErrorReported;
use rustc::session::config::{OutputFilenames, PrintRequest};
use rustc::ty::TyCtxt;
use rustc::ty::query::Providers;
use rustc::middle::cstore::{EncodedMetadata, MetadataLoader};
use rustc::dep_graph::DepGraph;
use rustc_data_structures::sync::Lrc;
use rustc_data_structures::owning_ref::{self, OwningRef};
use rustc_data_structures::rustc_erase_owner;

pub use rustc_data_structures::sync::MetadataRef;

pub trait CodegenBackend {
    fn init(&self, _sess: &Session) {}
    fn print(&self, _req: PrintRequest, _sess: &Session) {}
    fn target_features(&self, _sess: &Session) -> Vec<Symbol> { vec![] }
    fn print_passes(&self) {}
    fn print_version(&self) {}
    fn diagnostics(&self) -> &[(&'static str, &'static str)] { &[] }

    fn metadata_loader(&self) -> Box<dyn MetadataLoader + Sync>;
    fn provide(&self, _providers: &mut Providers<'_>);
    fn provide_extern(&self, _providers: &mut Providers<'_>);
    fn codegen_crate<'a, 'tcx>(
        &self,
        tcx: TyCtxt<'a, 'tcx, 'tcx>,
        metadata: EncodedMetadata,
        need_metadata_module: bool,
        rx: mpsc::Receiver<Box<dyn Any + Send>>
    ) -> Box<dyn Any>;

    /// This is called on the returned `Box<dyn Any>` from `codegen_backend`
    ///
    /// # Panics
    ///
    /// Panics when the passed `Box<dyn Any>` was not returned by `codegen_backend`.
    fn join_codegen_and_link(
        &self,
        ongoing_codegen: Box<dyn Any>,
        sess: &Session,
        dep_graph: &DepGraph,
        outputs: &OutputFilenames,
    ) -> Result<(), ErrorReported>;
}

pub struct MetadataOnlyCodegenBackend;

impl CodegenBackend for MetadataOnlyCodegenBackend {
    fn init(&self, sess: &Session) {
        for cty in sess.opts.crate_types.iter() {
            match *cty {
                CrateType::Executable => {}
                _ => {
                    sess.warn(&format!(
                        "Rustc codegen cranelift doesn't support output type {}",
                        cty
                    ));
                }
            }
        }
    }

    fn metadata_loader(&self) -> Box<dyn MetadataLoader + Sync> {
        Box::new(RlibMetadataLoader)
    }

    fn provide(&self, providers: &mut Providers<'_>) {
        crate::symbol_names::provide(providers);
        crate::symbol_export::provide(providers);

        providers.target_features_whitelist = |_tcx, _cnum| Lrc::new(Default::default());
    }
    fn provide_extern(&self, providers: &mut Providers<'_>) {
        crate::symbol_export::provide_extern(providers);
    }

    fn codegen_crate<'a, 'tcx>(
        &self,
        _tcx: TyCtxt<'a, 'tcx, 'tcx>,
        _metadata: EncodedMetadata,
        _need_metadata_module: bool,
        _rx: mpsc::Receiver<Box<dyn Any + Send>>,
    ) -> Box<dyn Any> {
        unimplemented!();
    }

    fn join_codegen_and_link(
        &self,
        _res: Box<dyn Any>,
        _sess: &Session,
        _dep_graph: &DepGraph,
        _outputs: &OutputFilenames,
    ) -> Result<(), ErrorReported> {
        unreachable!();
    }
}

struct RlibMetadataLoader;

impl MetadataLoader for RlibMetadataLoader {
    fn get_rlib_metadata(
        &self,
        _target: &rustc_target::spec::Target,
        path: &Path,
    ) -> Result<owning_ref::ErasedBoxRef<[u8]>, String> {
        let mut archive = ar::Archive::new(File::open(path).map_err(|e| format!("{:?}", e))?);
        // Iterate over all entries in the archive:
        while let Some(entry_result) = archive.next_entry() {
            let mut entry = entry_result.map_err(|e| format!("{:?}", e))?;
            if entry.header().identifier() == b"rust.metadata.bin" {
                let mut buf = Vec::new();
                ::std::io::copy(&mut entry, &mut buf).map_err(|e| format!("{:?}", e))?;
                let buf: OwningRef<Vec<u8>, [u8]> = OwningRef::new(buf).into();
                return Ok(rustc_erase_owner!(buf.map_owner_box()));
            }
        }

        Err("couldn't find metadata entry".to_string())
        //self.get_dylib_metadata(target, path)
    }

    fn get_dylib_metadata(
        &self,
        _target: &rustc_target::spec::Target,
        _path: &Path,
    ) -> Result<owning_ref::ErasedBoxRef<[u8]>, String> {
        Err("dylib metadata loading is not yet supported".to_string())
    }
}
