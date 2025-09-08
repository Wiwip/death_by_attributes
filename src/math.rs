use num_traits::Bounded;

pub trait AbsDiff<Rhs = Self> {
    type Output;
    fn abs_diff(self, rhs: Rhs) -> Self::Output;
    fn are_different(self, rhs: Rhs) -> bool;
}

// Macro to implement AbsSub for signed integers
macro_rules! impl_abs_diff_for_signed {
    ($($t:ty),*) => {
        $(
            impl AbsDiff for $t {
                type Output = $t;
                fn abs_diff(self, rhs: $t) -> $t {
                    (self - rhs).abs()
                }
                fn are_different(self, rhs: $t) -> bool {
                    self.abs_diff(rhs) >= Self::Output::min_value().try_into().unwrap()
                }
            }
        )*
    };
}

// Macro to implement AbsSub for unsigned integers
macro_rules! impl_abs_diff_for_unsigned {
    ($($t:ty),*) => {
        $(
            impl AbsDiff for $t {
                type Output = $t;
                fn abs_diff(self, rhs: $t) -> $t {
                    self.abs_diff(rhs)
                }
                fn are_different(self, rhs: $t) -> bool {
                    self.abs_diff(rhs) >= Self::Output::min_value()
                }
            }
        )*
    };
}


// Implement the trait for all the primitive integer types
impl_abs_diff_for_signed!(i8, i16, i32, i64, i128, isize);
impl_abs_diff_for_signed!(f32, f64);
impl_abs_diff_for_unsigned!(u8, u16, u32, u64, u128, usize);
