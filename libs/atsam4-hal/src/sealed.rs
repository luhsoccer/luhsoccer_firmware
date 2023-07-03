#![allow(clippy::no_effect)]

#[allow(dead_code)]
#[allow(path_statements)]
pub(crate) const fn not_one_or_two<const N: usize>() {
    Assert::<N, 1>::NOT_EQ;
    Assert::<N, 2>::NOT_EQ;
}

#[allow(dead_code)]
#[allow(path_statements)]
pub(crate) const fn smaller_than_or_eq<const N: usize, const MAX: usize>() {
    Assert::<N, MAX>::LESS_EQ;
}

#[allow(dead_code)]
#[allow(path_statements)]
pub(crate) const fn equal<const N: usize, const M: usize>() {
    Assert::<N, M>::EQ;
}

#[allow(dead_code)]
pub struct Assert<const L: usize, const R: usize>;

#[allow(dead_code)]
impl<const L: usize, const R: usize> Assert<L, R> {
    /// Const assert hack
    pub const GREATER_EQ: usize = L - R;

    /// Const assert hack
    pub const LESS_EQ: usize = R - L;

    /// Const assert hack
    #[allow(clippy::erasing_op)]
    pub const NOT_EQ: isize = 0 / (R as isize - L as isize);

    /// Const assert hack
    pub const EQ: usize = (R - L) + (L - R);

    /// Const assert hack
    pub const GREATER: usize = L - R - 1;

    /// Const assert hack
    pub const LESS: usize = R - L - 1;

    /// Const assert hack
    pub const POWER_OF_TWO: usize = 0 - (L & (L - 1));
}
