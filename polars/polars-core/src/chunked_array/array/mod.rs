//! Special fixed-size-list utility methods

mod iterator;

use crate::prelude::*;

impl ArrayChunked {
    pub fn to_physical(&mut self, inner_dtype: DataType) {
        debug_assert_eq!(inner_dtype.to_physical(), self.inner_dtype());
        let fld = Arc::make_mut(&mut self.field);
        fld.coerce(DataType::List(Box::new(inner_dtype)))
    }

    /// Get the inner values as `Series`
    pub fn get_inner(&self) -> Series {
        let ca = self.rechunk();
        let inner_dtype = self.inner_dtype().to_arrow();
        let arr = ca.downcast_iter().next().unwrap();
        unsafe {
            Series::try_from_arrow_unchecked(
                self.name(),
                vec![(arr.values()).clone()],
                &inner_dtype,
            )
            .unwrap()
        }
    }

    /// Ignore the list indices and apply `func` to the inner type as `Series`.
    pub fn apply_to_inner(
        &self,
        func: &dyn Fn(Series) -> PolarsResult<Series>,
    ) -> PolarsResult<ArrayChunked> {
        // generated Series will have wrong length otherwise.
        let ca = self.rechunk();
        let inner_dtype = self.inner_dtype().to_arrow();

        let chunks = ca.downcast_iter().map(|arr| {
            let elements = unsafe { Series::try_from_arrow_unchecked(self.name(), vec![(*arr.values()).clone()], &inner_dtype).unwrap() } ;

            let expected_len = elements.len();
            let out: Series = func(elements)?;
            polars_ensure!(
                out.len() == expected_len,
                ComputeError: "the function should apply element-wise, it removed elements instead"
            );
            let out = out.rechunk();
            let values = out.chunks()[0].clone();

            let inner_dtype = FixedSizeListArray::default_datatype(out.dtype().to_arrow(), ca.width());
            let arr = FixedSizeListArray::new(
                inner_dtype,
                values,
                arr.validity().cloned(),
            );
            Ok(Box::new(arr) as ArrayRef)
        }).collect::<PolarsResult<Vec<_>>>()?;

        unsafe { Ok(ArrayChunked::from_chunks(self.name(), chunks)) }
    }
}
