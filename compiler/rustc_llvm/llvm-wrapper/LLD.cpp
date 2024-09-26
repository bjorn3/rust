#include "lld/Common/Driver.h"

LLD_HAS_DRIVER(elf)
LLD_HAS_DRIVER(wasm)

extern "C" int RustRunLld(int argc, char **argv) {
    llvm::ArrayRef<const char *> args(argv, argv + argc);
    auto r = lld::lldMain(args, llvm::outs(), llvm::errs(),
                         {{lld::Gnu, &lld::elf::link}, {lld::Wasm, &lld::wasm::link}});
    return r.retCode;
}
