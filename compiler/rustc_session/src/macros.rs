/// Derivable trait for enums with no fields (i.e. C-style enums) that want to
/// allow iteration over a list of all variant values.
pub(crate) trait AllVariants: Copy + 'static {
    const ALL_VARIANTS: &[Self];
}
