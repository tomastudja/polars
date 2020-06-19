//! # Series
//! The columnar data type for a DataFrame.
//!
//! ## Arithmetic
//!
//! You can do standard arithmetic on series.
//! ```
//! # use polars::prelude::*;
//! let s: Series = [1, 2, 3].iter().collect();
//! let out_add = &s + &s;
//! let out_sub = &s - &s;
//! let out_div = &s / &s;
//! let out_mul = &s * &s;
//! ```
//!
//! Or with series and numbers.
//!
//! ```
//! # use polars::prelude::*;
//! let s: Series = (1..3).collect();
//! let out_add_one = &s + 1;
//! let out_multiply = &s * 10;
//!
//! // Could not overload left hand side operator.
//! let out_divide = 1.div(&s);
//! let out_add = 1.add(&s);
//! let out_subtract = 1.sub(&s);
//! let out_multiply = 1.mul(&s);
//! ```
//!
//! ## Comparison
//! You can obtain boolean mask by comparing series.
//!
//! ```
//! # use polars::prelude::*;
//! use itertools::Itertools;
//! let s = Series::init("dollars", [1, 2, 3].as_ref());
//! let mask = s.eq(1).expect("could not compare types");
//! let valid = [true, false, false].iter();
//! assert!(mask
//!     .iter()
//!     .map(|opt_bool| opt_bool.unwrap()) // option, because series can be null
//!     .zip(valid)
//!     .all(|(a, b)| a == *b))
//! ```
//!
//! See all the comparison operators in the [CmpOps trait](../chunked_array/comparison/trait.CmpOps.html)
//!
//! ## Iterators
//! The Series variants contain differently typed [ChunkedArray's](../chunked_array/struct.ChunkedArray.html).
//! These structs can be turned into iterators, making it possible to use any function/ closure you want
//! on a Series.
//!
//! These iteratiors return an `Option<T>` because the values of a series may be null.
//!
//! ```
//! use polars::prelude::*;
//! let pi = 3.14;
//! let s = Series::init("angle", [2f32 * pi, pi, 1.5 * pi].as_ref());
//! let s_cos: Series = s.f32()
//!                     .expect("series was not an f32 dtype")
//!                     .iter()
//!                     .map(|opt_angle| opt_angle.map(|angle| angle.cos()))
//!                     .collect();
//! ```

use super::chunked_array::ChunkedArray;
use crate::datatypes::{
    AnyType, Date32Chunked, Date64Chunked, DurationNsChunked, Float32Chunked, Float64Chunked,
    Int32Chunked, Int64Chunked, PolarsDataType, Time64NsChunked, UInt32Chunked, Utf8Chunked,
};
use crate::series::chunked_array::{ChunkOps, SeriesOps};
use crate::{
    datatypes,
    datatypes::{ArrowDataType, BooleanChunked},
    error::{PolarsError, Result},
};
use arrow::array::ArrayRef;
use arrow::compute::TakeOptions;
use arrow::datatypes::{ArrowPrimitiveType, Field};
use std::mem;
use std::ops::Deref;

#[derive(Clone)]
pub enum Series {
    UInt32(ChunkedArray<datatypes::UInt32Type>),
    Int32(ChunkedArray<datatypes::Int32Type>),
    Int64(ChunkedArray<datatypes::Int64Type>),
    Float32(ChunkedArray<datatypes::Float32Type>),
    Float64(ChunkedArray<datatypes::Float64Type>),
    Utf8(ChunkedArray<datatypes::Utf8Type>),
    Bool(ChunkedArray<datatypes::BooleanType>),
    Date32(ChunkedArray<datatypes::Date32Type>),
    Date64(ChunkedArray<datatypes::Date64Type>),
    Time64Ns(ChunkedArray<datatypes::Time64NanosecondType>),
    DurationNs(ChunkedArray<datatypes::DurationNanosecondType>),
}

#[macro_export]
macro_rules! apply_method_all_series {
    ($self:ident, $method:ident, $($args:ident),*) => {
        match $self {
            Series::UInt32(a) => a.$method($($args),*),
            Series::Int32(a) => a.$method($($args),*),
            Series::Int64(a) => a.$method($($args),*),
            Series::Float32(a) => a.$method($($args),*),
            Series::Float64(a) => a.$method($($args),*),
            Series::Utf8(a) => a.$method($($args),*),
            Series::Bool(a) => a.$method($($args),*),
            Series::Date32(a) => a.$method($($args),*),
            Series::Date64(a) => a.$method($($args),*),
            Series::Time64Ns(a) => a.$method($($args),*),
            Series::DurationNs(a) => a.$method($($args),*),
        }
    }
}

// doesn't include Bool and Utf8
#[macro_export]
macro_rules! apply_method_arrowprimitive_series {
    ($self:ident, $method:ident, $($args:ident),*) => {
        match $self {
            Series::UInt32(a) => a.$method($($args),*),
            Series::Int32(a) => a.$method($($args),*),
            Series::Int64(a) => a.$method($($args),*),
            Series::Float32(a) => a.$method($($args),*),
            Series::Float64(a) => a.$method($($args),*),
            Series::Date32(a) => a.$method($($args),*),
            Series::Date64(a) => a.$method($($args),*),
            Series::Time64Ns(a) => a.$method($($args),*),
            Series::DurationNs(a) => a.$method($($args),*),
            _ => unimplemented!(),
        }
    }
}

macro_rules! apply_method_and_return {
    ($self:ident, $method:ident, [$($args:expr),*], $($opt_question_mark:tt)*) => {
        match $self {
            Series::UInt32(a) => Series::UInt32(a.$method($($args),*)$($opt_question_mark)*),
            Series::Int32(a) => Series::Int32(a.$method($($args),*)$($opt_question_mark)*),
            Series::Int64(a) => Series::Int64(a.$method($($args),*)$($opt_question_mark)*),
            Series::Float32(a) => Series::Float32(a.$method($($args),*)$($opt_question_mark)*),
            Series::Float64(a) => Series::Float64(a.$method($($args),*)$($opt_question_mark)*),
            Series::Utf8(a) => Series::Utf8(a.$method($($args),*)$($opt_question_mark)*),
            Series::Bool(a) => Series::Bool(a.$method($($args),*)$($opt_question_mark)*),
            Series::Date32(a) => Series::Date32(a.$method($($args),*)$($opt_question_mark)*),
            Series::Date64(a) => Series::Date64(a.$method($($args),*)$($opt_question_mark)*),
            Series::Time64Ns(a) => Series::Time64Ns(a.$method($($args),*)$($opt_question_mark)*),
            Series::DurationNs(a) => Series::DurationNs(a.$method($($args),*)$($opt_question_mark)*),
            _ => unimplemented!(),
        }
    }
}

macro_rules! unpack_series {
    ($self:ident, $variant:ident) => {
        if let Series::$variant(ca) = $self {
            Ok(ca)
        } else {
            Err(PolarsError::DataTypeMisMatch)
        }
    };
}

impl Series {
    /// Name of series.
    pub fn name(&self) -> &str {
        apply_method_all_series!(self, name,)
    }

    /// Rename series.
    pub fn rename(&mut self, name: &str) {
        apply_method_all_series!(self, rename, name)
    }

    /// Get field (used in schema)
    pub fn field(&self) -> &Field {
        apply_method_all_series!(self, ref_field,)
    }

    pub fn dtype(&self) -> &ArrowDataType {
        self.field().data_type()
    }

    /// Unpack to ChunkedArray
    /// ```
    /// # use polars::prelude::*;
    /// let s: Series = [1, 2, 3].iter().collect();
    /// let s_squared: Series = s.i32()
    ///     .unwrap()
    ///     .iter()
    ///     .map(|opt_v| {
    ///         match opt_v {
    ///             Some(v) => Some(v * v),
    ///             None => None, // null value
    ///         }
    /// }).collect();
    /// ```
    pub fn i32(&self) -> Result<&Int32Chunked> {
        unpack_series!(self, Int32)
    }

    /// Unpack to ChunkedArray
    pub fn i64(&self) -> Result<&Int64Chunked> {
        unpack_series!(self, Int64)
    }

    /// Unpack to ChunkedArray
    pub fn f32(&self) -> Result<&Float32Chunked> {
        unpack_series!(self, Float32)
    }

    /// Unpack to ChunkedArray
    pub fn f64(&self) -> Result<&Float64Chunked> {
        unpack_series!(self, Float64)
    }

    /// Unpack to ChunkedArray
    pub fn u32(&self) -> Result<&UInt32Chunked> {
        unpack_series!(self, UInt32)
    }

    /// Unpack to ChunkedArray
    pub fn bool(&self) -> Result<&BooleanChunked> {
        unpack_series!(self, Bool)
    }

    /// Unpack to ChunkedArray
    pub fn utf8(&self) -> Result<&Utf8Chunked> {
        unpack_series!(self, Utf8)
    }

    /// Unpack to ChunkedArray
    pub fn date32(&self) -> Result<&Date32Chunked> {
        unpack_series!(self, Date32)
    }

    /// Unpack to ChunkedArray
    pub fn date64(&self) -> Result<&Date64Chunked> {
        unpack_series!(self, Date64)
    }

    /// Unpack to ChunkedArray
    pub fn time64ns(&self) -> Result<&Time64NsChunked> {
        unpack_series!(self, Time64Ns)
    }

    /// Unpack to ChunkedArray
    pub fn duration_ns(&self) -> Result<&DurationNsChunked> {
        unpack_series!(self, DurationNs)
    }

    pub fn append_array(&mut self, other: ArrayRef) -> Result<()> {
        apply_method_all_series!(self, append_array, other)
    }

    pub fn as_series_ops(&self) -> &dyn SeriesOps {
        match self {
            Series::UInt32(arr) => arr,
            Series::Int32(arr) => arr,
            Series::Int64(arr) => arr,
            Series::Float32(arr) => arr,
            Series::Float64(arr) => arr,
            Series::Utf8(arr) => arr,
            Series::Date32(arr) => arr,
            Series::Date64(arr) => arr,
            Series::Time64Ns(arr) => arr,
            Series::Bool(arr) => arr,
            Series::DurationNs(arr) => arr,
        }
    }

    /// Take `num_elements`.
    pub fn limit(&self, num_elements: usize) -> Result<Self> {
        Ok(apply_method_and_return!(self, limit, [num_elements], ?))
    }

    /// Filter by boolean mask.
    pub fn filter<T: AsRef<BooleanChunked>>(&self, filter: T) -> Result<Self> {
        Ok(apply_method_and_return!(self, filter, [filter.as_ref()], ?))
    }

    /// Take by index.
    pub fn take<T: AsRef<UInt32Chunked>>(
        &self,
        indices: T,
        options: Option<TakeOptions>,
    ) -> Result<Self> {
        Ok(apply_method_and_return!(self, take, [indices.as_ref(), options], ?))
    }

    /// Get length of series.
    pub fn len(&self) -> usize {
        apply_method_all_series!(self, len,)
    }

    /// Aggregate all chunks to a contiguous array of memory.
    pub fn rechunk(&mut self) {
        apply_method_all_series!(self, rechunk,)
    }

    /// Cast to an arrow primitive type.
    pub fn cast<N>(&self) -> Result<Self>
    where
        N: ArrowPrimitiveType,
    {
        let s = match self {
            Series::UInt32(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Int32(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Int64(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Float32(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Float64(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Date32(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Date64(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::Time64Ns(arr) => pack_ca_to_series(arr.cast::<N>()?),
            Series::DurationNs(arr) => pack_ca_to_series(arr.cast::<N>()?),
            _ => return Err(PolarsError::DataTypeMisMatch),
        };
        Ok(s)
    }

    pub fn get(&self, index: usize) -> AnyType {
        apply_method_all_series!(self, get, index)
    }

    pub fn sort(&mut self) {
        apply_method_arrowprimitive_series!(self, sort_in_place,)
    }

    pub fn argsort(&self) -> UInt32Chunked {
        apply_method_arrowprimitive_series!(self, argsort,)
    }
}

fn pack_ca_to_series<N: ArrowPrimitiveType>(ca: ChunkedArray<N>) -> Series {
    unsafe {
        match N::get_data_type() {
            ArrowDataType::UInt32 => Series::UInt32(mem::transmute(ca)),
            ArrowDataType::Int32 => Series::Int32(mem::transmute(ca)),
            ArrowDataType::Int64 => Series::Int64(mem::transmute(ca)),
            ArrowDataType::Float32 => Series::Float32(mem::transmute(ca)),
            ArrowDataType::Float64 => Series::Float64(mem::transmute(ca)),
            ArrowDataType::Date32(_) => Series::Date32(mem::transmute(ca)),
            ArrowDataType::Date64(_) => Series::Date64(mem::transmute(ca)),
            ArrowDataType::Time64(datatypes::TimeUnit::Nanosecond) => {
                Series::Time64Ns(mem::transmute(ca))
            }
            ArrowDataType::Duration(datatypes::TimeUnit::Nanosecond) => {
                Series::DurationNs(mem::transmute(ca))
            }
            _ => unimplemented!(),
        }
    }
}

pub trait NamedFrom<T> {
    /// Initialize by name and values.
    fn init(name: &str, _: T) -> Self;
}

macro_rules! impl_named_from {
    ($type:ty, $series_var:ident, $method:ident) => {
        impl NamedFrom<&[$type]> for Series {
            fn init(name: &str, v: &[$type]) -> Self {
                Series::$series_var(ChunkedArray::$method(name, v))
            }
        }
    };
}

impl_named_from!(&str, Utf8, new_utf8_from_slice);
impl_named_from!(String, Utf8, new_utf8_from_slice);
impl_named_from!(bool, Bool, new_from_slice);
impl_named_from!(u32, UInt32, new_from_slice);
impl_named_from!(i32, Int32, new_from_slice);
impl_named_from!(i64, Int64, new_from_slice);
impl_named_from!(f32, Float32, new_from_slice);
impl_named_from!(f64, Float64, new_from_slice);

macro_rules! impl_as_ref_ca {
    ($type:ident, $series_var:ident) => {
        impl AsRef<ChunkedArray<datatypes::$type>> for Series {
            fn as_ref(&self) -> &ChunkedArray<datatypes::$type> {
                match self {
                    Series::$series_var(a) => a,
                    _ => unimplemented!(),
                }
            }
        }
    };
}

impl_as_ref_ca!(UInt32Type, UInt32);
impl_as_ref_ca!(Int32Type, Int32);
impl_as_ref_ca!(Int64Type, Int64);
impl_as_ref_ca!(Float32Type, Float32);
impl_as_ref_ca!(Float64Type, Float64);
impl_as_ref_ca!(BooleanType, Bool);
impl_as_ref_ca!(Utf8Type, Utf8);

macro_rules! impl_as_mut_ca {
    ($type:ident, $series_var:ident) => {
        impl AsMut<ChunkedArray<datatypes::$type>> for Series {
            fn as_mut(&mut self) -> &mut ChunkedArray<datatypes::$type> {
                match self {
                    Series::$series_var(a) => a,
                    _ => unimplemented!(),
                }
            }
        }
    };
}

impl_as_mut_ca!(UInt32Type, UInt32);
impl_as_mut_ca!(Int32Type, Int32);
impl_as_mut_ca!(Int64Type, Int64);
impl_as_mut_ca!(Float32Type, Float32);
impl_as_mut_ca!(Float64Type, Float64);
impl_as_mut_ca!(BooleanType, Bool);
impl_as_mut_ca!(Utf8Type, Utf8);

mod test {
    use crate::prelude::*;

    #[test]
    fn cast() {
        let ar = ChunkedArray::<Int32Type>::new_from_slice("a", &[1, 2]);
        let s = Series::Int32(ar);
        let s2 = s.cast::<Int64Type>().unwrap();
        match s2 {
            Series::Int64(_) => assert!(true),
            _ => assert!(false),
        }
        let s2 = s.cast::<Float32Type>().unwrap();
        match s2 {
            Series::Float32(_) => assert!(true),
            _ => assert!(false),
        }
    }
}
