//! Internal interface for communicating between a `proc_macro` client
//! (a proc macro crate) and a `proc_macro` server (a compiler front-end).
//!
//! Serialization (with C ABI buffers) and unique integer handles are employed
//! to allow safely interfacing between two copies of `proc_macro` built
//! (from the same source) by different compilers with potentially mismatching
//! Rust ABIs (e.g., stage0/bin/rustc vs stage1/bin/rustc during bootstrap).

#![deny(unsafe_code)]

use std::hash::Hash;
use std::ops::{Bound, Range};
use std::sync::Once;
use std::{fmt, mem, panic, thread};

use crate::{Delimiter, Level};

/// Higher-order macro describing the server RPC API, allowing automatic
/// generation of type-safe Rust APIs, both client-side and server-side.
///
/// `with_api!(my_macro, MyTokenStream, MySpan, MySymbol)` expands to:
/// ```rust,ignore (pseudo-code)
/// my_macro! {
///     fn ts_clone(stream: &MyTokenStream) -> MyTokenStream;
///     fn span_debug(span: &MySpan) -> String;
///     // ...
/// }
/// ```
///
/// The second (`TokenStream`), third (`Span`) and fourth (`Symbol`)
/// argument serve to customize the argument/return types that need
/// special handling, to enable several different representations of
/// these types.
macro_rules! with_api {
    ($m:ident, $TokenStream: path, $Span: path, $Symbol: path) => {
        $m! {
            fn injected_env_var(var: &str) -> Option<String>;
            fn track_env_var(var: &str, value: Option<&str>);
            fn track_path(path: &str);
            fn literal_from_str(s: &str) -> Result<Literal<Span<$Span>, Symbol<$Symbol>>, ()>;
            fn emit_diagnostic(diagnostic: Diagnostic<Span<$Span>>);

            fn ts_drop(stream: TokenStream<$TokenStream>);
            fn ts_clone(stream: &TokenStream<$TokenStream>) -> TokenStream<$TokenStream>;
            fn ts_is_empty(stream: &TokenStream<$TokenStream>) -> bool;
            fn ts_expand_expr(stream: &TokenStream<$TokenStream>) -> Result<TokenStream<$TokenStream>, ()>;
            fn ts_from_str(src: &str) -> TokenStream<$TokenStream>;
            fn ts_to_string(stream: &TokenStream<$TokenStream>) -> String;
            fn ts_from_token_tree(
                tree: TokenTree<TokenStream<$TokenStream>, Span<$Span>, Symbol<$Symbol>>,
            ) -> TokenStream<$Span>;
            fn ts_concat_trees(
                base: Option<TokenStream<$TokenStream>>,
                trees: Vec<TokenTree<TokenStream<$TokenStream>, Span<$Span>, Symbol<$Symbol>>>,
            ) -> TokenStream<$TokenStream>;
            fn ts_concat_streams(
                base: Option<TokenStream<$TokenStream>>,
                streams: Vec<TokenStream<$TokenStream>>,
            ) -> TokenStream<$TokenStream>;
            fn ts_into_trees(
                stream: TokenStream<$TokenStream>
            ) -> Vec<TokenTree<TokenStream<$TokenStream>, Span<$Span>, Symbol<$Symbol>>>;

            fn span_debug(span: Span<$Span>) -> String;
            fn span_parent(span: Span<$Span>) -> Option<Span<$Span>>;
            fn span_source(span: Span<$Span>) -> Span<$Span>;
            fn span_byte_range(span: Span<$Span>) -> Range<usize>;
            fn span_start(span: Span<$Span>) -> Span<$Span>;
            fn span_end(span: Span<$Span>) -> Span<$Span>;
            fn span_line(span: Span<$Span>) -> usize;
            fn span_column(span: Span<$Span>) -> usize;
            fn span_file(span: Span<$Span>) -> String;
            fn span_local_file(span: Span<$Span>) -> Option<String>;
            fn span_join(span: Span<$Span>, other: Span<$Span>) -> Option<Span<$Span>>;
            fn span_subspan(span: Span<$Span>, start: Bound<usize>, end: Bound<usize>) -> Option<Span<$Span>>;
            fn span_resolved_at(span: Span<$Span>, at: Span<$Span>) -> Span<$Span>;
            fn span_source_text(span: Span<$Span>) -> Option<String>;
            fn span_save_span(span: Span<$Span>) -> usize;
            fn span_recover_proc_macro_span(id: usize) -> Span<$Span>;

            fn symbol_normalize_and_validate_ident(string: &str) -> Result<Symbol<$Symbol>, ()>;
        }
    };
}

pub(crate) struct Methods;

pub(crate) struct TokenStream<T>(T);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Span<T>(T);

#[derive(Copy, Clone)]
pub(crate) struct Symbol<T>(pub(crate) T);

#[allow(unsafe_code)]
mod arena;
#[allow(unsafe_code)]
mod buffer;
#[deny(unsafe_code)]
pub mod client;
#[allow(unsafe_code)]
mod closure;
#[forbid(unsafe_code)]
mod fxhash;
#[forbid(unsafe_code)]
mod handle;
#[macro_use]
#[forbid(unsafe_code)]
mod rpc;
#[allow(unsafe_code)]
mod selfless_reify;
#[forbid(unsafe_code)]
pub mod server;
#[allow(unsafe_code)]
mod symbol;

use buffer::Buffer;
pub use rpc::PanicMessage;
use rpc::{Decode, Encode};

/// Configuration for establishing an active connection between a server and a
/// client.  The server creates the bridge config (`run_server` in `server.rs`),
/// then passes it to the client through the function pointer in the `run` field
/// of `client::Client`. The client constructs a local `Bridge` from the config
/// in TLS during its execution (`Bridge::{enter, with}` in `client.rs`).
#[repr(C)]
pub struct BridgeConfig<'a> {
    /// Buffer used to pass initial input to the client.
    input: Buffer,

    /// Server-side function that the client uses to make requests.
    dispatch: closure::Closure<'a>,

    /// If 'true', always invoke the default panic hook
    force_show_panics: bool,
}

impl !Send for BridgeConfig<'_> {}
impl !Sync for BridgeConfig<'_> {}

pub trait Types {
    type TokenStream: 'static + Clone;
    type Span: 'static + Copy + Eq + Hash;
    type Symbol: 'static;
}

macro_rules! declare_tags {
    (
        $(fn $method:ident($($arg:ident: $arg_ty:ty),* $(,)?) $(-> $ret_ty:ty)?;)*
    ) => {
        #[allow(non_camel_case_types)]
        pub(super) enum ApiTags {
            $($method),*
        }
        rpc_encode_decode!(enum ApiTags { $($method),* });
    }
}
with_api!(declare_tags, __, __, __);

rpc_encode_decode!(
    enum Delimiter {
        Parenthesis,
        Brace,
        Bracket,
        None,
    }
);
rpc_encode_decode!(
    enum Level {
        Error,
        Warning,
        Note,
        Help,
    }
);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LitKind {
    Byte,
    Char,
    Integer,
    Float,
    Str,
    StrRaw(u8),
    ByteStr,
    ByteStrRaw(u8),
    CStr,
    CStrRaw(u8),
    // This should have an `ErrorGuaranteed`, except that type isn't available
    // in this crate. (Imagine it is there.) Hence the `WithGuar` suffix. Must
    // only be constructed in `LitKind::from_internal`, where an
    // `ErrorGuaranteed` is available.
    ErrWithGuar,
}

rpc_encode_decode!(
    enum LitKind {
        Byte,
        Char,
        Integer,
        Float,
        Str,
        StrRaw(n),
        ByteStr,
        ByteStrRaw(n),
        CStr,
        CStrRaw(n),
        ErrWithGuar,
    }
);

rpc_encode_decode!(
    enum Bound<T> {
        Included(x),
        Excluded(x),
        Unbounded,
    }
);

rpc_encode_decode!(
    enum Option<T> {
        Some(t),
        None,
    }
);

rpc_encode_decode!(
    enum Result<T, E> {
        Ok(t),
        Err(e),
    }
);

#[derive(Copy, Clone)]
pub struct DelimSpan<Span> {
    pub open: Span,
    pub close: Span,
    pub entire: Span,
}

impl<Span: Copy> DelimSpan<Span> {
    pub fn from_single(span: Span) -> Self {
        DelimSpan { open: span, close: span, entire: span }
    }
}

rpc_encode_decode!(struct DelimSpan<Span> { open, close, entire });

#[derive(Clone)]
pub struct Group<TokenStream, Span> {
    pub delimiter: Delimiter,
    pub stream: Option<TokenStream>,
    pub span: DelimSpan<Span>,
}

rpc_encode_decode!(struct Group<TokenStream, Span> { delimiter, stream, span });

#[derive(Clone)]
pub struct Punct<Span> {
    pub ch: u8,
    pub joint: bool,
    pub span: Span,
}

rpc_encode_decode!(struct Punct<Span> { ch, joint, span });

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Ident<Span, Symbol> {
    pub sym: Symbol,
    pub is_raw: bool,
    pub span: Span,
}

rpc_encode_decode!(struct Ident<Span, Symbol> { sym, is_raw, span });

#[derive(Clone, Eq, PartialEq)]
pub struct Literal<Span, Symbol> {
    pub kind: LitKind,
    pub symbol: Symbol,
    pub suffix: Option<Symbol>,
    pub span: Span,
}

rpc_encode_decode!(struct Literal<Sp, Sy> { kind, symbol, suffix, span });

#[derive(Clone)]
pub enum TokenTree<TokenStream, Span, Symbol> {
    Group(Group<TokenStream, Span>),
    Punct(Punct<Span>),
    Ident(Ident<Span, Symbol>),
    Literal(Literal<Span, Symbol>),
}

rpc_encode_decode!(
    enum TokenTree<TokenStream, Span, Symbol> {
        Group(tt),
        Punct(tt),
        Ident(tt),
        Literal(tt),
    }
);

#[derive(Clone, Debug)]
pub struct Diagnostic<Span> {
    pub level: Level,
    pub message: String,
    pub spans: Vec<Span>,
    pub children: Vec<Diagnostic<Span>>,
}

rpc_encode_decode!(
    struct Diagnostic<Span> { level, message, spans, children }
);

/// Globals provided alongside the initial inputs for a macro expansion.
/// Provides values such as spans which are used frequently to avoid RPC.
#[derive(Clone)]
pub struct ExpnGlobals<T> {
    pub def_site: Span<T>,
    pub call_site: Span<T>,
    pub mixed_site: Span<T>,
}

rpc_encode_decode!(
    struct ExpnGlobals<T> { def_site, call_site, mixed_site }
);

rpc_encode_decode!(
    struct Range<T> { start, end }
);
