pub use crate::{
    datatypes,
    datatypes::*,
    error::{PolarsError, Result},
    frame::{
        csv::{CsvReader, CsvWriter},
        DataFrame,
    },
    series::{
        arithmetic::LhsNumOps,
        chunked_array::{
            aggregate::Agg,
            builder::{PrimitiveChunkedBuilder, Utf8ChunkedBuilder},
            chunkops::ChunkOps,
            comparison::{CmpOps, ForceCmpOps},
            take::{Take, TakeIndex},
            unique::Unique,
            ChunkedArray, Downcast,
        },
        NamedFrom, Series,
    },
    testing::*,
};

#[cfg(test)]
pub(crate) fn create_df() -> DataFrame {
    let s0 = Series::new("days", [0, 1, 2, 3, 4].as_ref());
    let s1 = Series::new("temp", [22.1, 19.9, 7., 2., 3.].as_ref());
    DataFrame::new(vec![s0, s1]).unwrap()
}
