// arrayref — patched for nightly compatibility (2024+)
// Original: https://github.com/droundy/arrayref
// Patch: fix macro ambiguity and deprecated ref patterns on nightly

#![no_std]
#![allow(unused_macros, unused_unsafe, clippy::all)]

/// Take a reference to a fixed-size subarray of a slice.
///
/// `array_ref!` is a macro that takes a slice and an offset and a length
/// and returns a `&[T; length]`.
#[macro_export]
macro_rules! array_ref {
    ($arr:expr, $offset:expr, $len:expr) => {{
        {
            #[inline]
            unsafe fn as_array<T>(slice: &[T]) -> &[T; $len] {
                &*(slice.as_ptr() as *const [T; $len])
            }
            let offset = $offset;
            let slice = & $arr[offset .. offset + $len];
            #[allow(unused_unsafe)]
            unsafe { as_array(slice) }
        }
    }};
}

/// Take a mutable reference to a fixed-size subarray of a slice.
///
/// `array_mut_ref!` is a macro that takes a mutable slice and an offset
/// and a length and returns a `&mut [T; length]`.
#[macro_export]
macro_rules! array_mut_ref {
    ($arr:expr, $offset:expr, $len:expr) => {{
        {
            #[inline]
            unsafe fn as_array_mut<T>(slice: &mut [T]) -> &mut [T; $len] {
                &mut *(slice.as_mut_ptr() as *mut [T; $len])
            }
            let offset = $offset;
            let slice = &mut $arr[offset .. offset + $len];
            #[allow(unused_unsafe)]
            unsafe { as_array_mut(slice) }
        }
    }};
}

/// Take multiple non-overlapping references from a slice.
///
/// `array_refs!` is a macro that takes a slice and a list of lengths
/// and returns a tuple of `&[T; length]` references.
#[macro_export]
macro_rules! array_refs {
    ($arr:expr, $($len:expr),+) => {{
        {
            let arr = $arr;
            let mut _offset = 0;
            ($({
                #[allow(unused_unsafe)]
                let slice = unsafe {
                    let s = &arr[_offset .. _offset + $len];
                    &*(s.as_ptr() as *const [_; $len])
                };
                _offset += $len;
                slice
            }),+)
        }
    }};
}

/// Take multiple non-overlapping mutable references from a mutable slice.
///
/// `mut_array_refs!` is a macro that takes a mutable slice and a list of
/// lengths and returns a tuple of `&mut [T; length]` references.
#[macro_export]
macro_rules! mut_array_refs {
    ($arr:expr, $($len:expr),+) => {{
        {
            let arr = $arr;
            let mut _offset = 0;
            ($({
                #[allow(unused_unsafe)]
                let slice = unsafe {
                    let s = &mut arr[_offset .. _offset + $len];
                    &mut *(s.as_mut_ptr() as *mut [_; $len])
                };
                _offset += $len;
                slice
            }),+)
        }
    }};
}
