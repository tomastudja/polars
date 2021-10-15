use crate::lazy::dsl::PyExpr;
use crate::prelude::PyDataType;
use crate::series::PySeries;
use crate::utils::str_to_polarstype;
use polars::prelude::*;
use pyo3::prelude::*;
use pyo3::types::PyList;

fn get_output_type(obj: &PyAny) -> Option<DataType> {
    match obj.is_none() {
        true => None,
        false => Some(obj.extract::<PyDataType>().unwrap().into()),
    }
}

pub(crate) fn call_lambda_with_series(
    py: Python,
    s: Series,
    lambda: &PyObject,
    polars_module: &PyObject,
) -> PyObject {
    let pypolars = polars_module.cast_as::<PyModule>(py).unwrap();

    // create a PySeries struct/object for Python
    let pyseries = PySeries::new(s);
    // Wrap this PySeries object in the python side Series wrapper
    let python_series_wrapper = pypolars
        .getattr("wrap_s")
        .unwrap()
        .call1((pyseries,))
        .unwrap();
    // call the lambda and get a python side Series wrapper
    match lambda.call1(py, (python_series_wrapper,)) {
        Ok(pyobj) => pyobj,
        Err(e) => panic!("python apply failed: {}", e.pvalue(py).to_string()),
    }
}

/// A python lambda taking two Series
pub(crate) fn binary_lambda(lambda: &PyObject, a: Series, b: Series) -> Result<Series> {
    let gil = Python::acquire_gil();
    let py = gil.python();
    // get the pypolars module
    let pypolars = PyModule::import(py, "polars").unwrap();
    // create a PySeries struct/object for Python
    let pyseries_a = PySeries::new(a);
    let pyseries_b = PySeries::new(b);

    // Wrap this PySeries object in the python side Series wrapper
    let python_series_wrapper_a = pypolars
        .getattr("wrap_s")
        .unwrap()
        .call1((pyseries_a,))
        .unwrap();
    let python_series_wrapper_b = pypolars
        .getattr("wrap_s")
        .unwrap()
        .call1((pyseries_b,))
        .unwrap();

    // call the lambda and get a python side Series wrapper
    let result_series_wrapper =
        match lambda.call1(py, (python_series_wrapper_a, python_series_wrapper_b)) {
            Ok(pyobj) => pyobj,
            Err(e) => panic!(
                "custom python function failed: {}",
                e.pvalue(py).to_string()
            ),
        };
    // unpack the wrapper in a PySeries
    let py_pyseries = result_series_wrapper
        .getattr(py, "_s")
        .expect("Could net get series attribute '_s'. Make sure that you return a Series object.");
    // Downcast to Rust
    let pyseries = py_pyseries.extract::<PySeries>(py).unwrap();
    // Finally get the actual Series
    Ok(pyseries.series)
}

pub fn binary_function(
    input_a: PyExpr,
    input_b: PyExpr,
    lambda: PyObject,
    output_type: &PyAny,
) -> PyExpr {
    let input_a = input_a.inner;
    let input_b = input_b.inner;

    let output_field = match output_type.is_none() {
        true => Field::new("binary_function", DataType::Null),
        false => {
            let str_repr = output_type.str().unwrap().to_str().unwrap();
            let data_type = str_to_polarstype(str_repr);
            Field::new("binary_function", data_type)
        }
    };

    let func = move |a: Series, b: Series| binary_lambda(&lambda, a, b);

    polars::lazy::dsl::map_binary(input_a, input_b, func, Some(output_field)).into()
}

pub fn map_single(
    pyexpr: &PyExpr,
    py: Python,
    lambda: PyObject,
    output_type: &PyAny,
    agg_list: bool,
) -> PyExpr {
    let output_type = get_output_type(output_type);
    // get the pypolars module
    // do the import outside of the function to prevent import side effects in a hot loop.
    let pypolars = PyModule::import(py, "polars").unwrap().to_object(py);

    let function = move |s: Series| {
        let gil = Python::acquire_gil();
        let py = gil.python();

        // this is a python Series
        let out = call_lambda_with_series(py, s, &lambda, &pypolars);

        // unpack the wrapper in a PySeries
        let py_pyseries = out.getattr(py, "_s").expect(
            "Could net get series attribute '_s'. \
                Make sure that you return a Series object from a custom function.",
        );
        // Downcast to Rust
        let pyseries = py_pyseries.extract::<PySeries>(py).unwrap();
        // Finally get the actual Series
        Ok(pyseries.series)
    };

    let output_map = GetOutput::map_field(move |fld| match output_type {
        Some(ref dt) => Field::new(fld.name(), dt.clone()),
        None => fld.clone(),
    });
    if agg_list {
        pyexpr.clone().inner.map_list(function, output_map).into()
    } else {
        pyexpr.clone().inner.map(function, output_map).into()
    }
}

pub(crate) fn call_lambda_with_series_slice(
    py: Python,
    s: &mut [Series],
    lambda: &PyObject,
    polars_module: &PyObject,
) -> PyObject {
    let pypolars = polars_module.cast_as::<PyModule>(py).unwrap();

    // create a PySeries struct/object for Python
    let iter = s.iter().map(|s| {
        let ps = PySeries::new(s.clone());

        // Wrap this PySeries object in the python side Series wrapper
        let python_series_wrapper = pypolars.getattr("wrap_s").unwrap().call1((ps,)).unwrap();

        python_series_wrapper
    });
    let wrapped_s = PyList::new(py, iter);

    // call the lambda and get a python side Series wrapper
    match lambda.call1(py, (wrapped_s,)) {
        Ok(pyobj) => pyobj,
        Err(e) => panic!("python apply failed: {}", e.pvalue(py).to_string()),
    }
}

pub fn map_mul(
    pyexpr: &[PyExpr],
    py: Python,
    lambda: PyObject,
    output_type: &PyAny,
    apply_groups: bool,
) -> PyExpr {
    let output_type = get_output_type(output_type);

    // get the pypolars module
    // do the import outside of the function to prevent import side effects in a hot loop.
    let pypolars = PyModule::import(py, "polars").unwrap().to_object(py);

    let function = move |s: &mut [Series]| {
        let gil = Python::acquire_gil();
        let py = gil.python();

        // this is a python Series
        let out = call_lambda_with_series_slice(py, s, &lambda, &pypolars);

        // unpack the wrapper in a PySeries
        let py_pyseries = out.getattr(py, "_s").expect(
            "Could net get series attribute '_s'. \
                Make sure that you return a Series object from a custom function.",
        );
        // Downcast to Rust
        let pyseries = py_pyseries.extract::<PySeries>(py).unwrap();
        // Finally get the actual Series
        Ok(pyseries.series)
    };

    let exprs = pyexpr.iter().map(|pe| pe.clone().inner).collect::<Vec<_>>();

    let output_map = GetOutput::map_field(move |fld| match output_type {
        Some(ref dt) => Field::new(fld.name(), dt.clone()),
        None => fld.clone(),
    });
    if apply_groups {
        polars::lazy::dsl::apply_mul(function, exprs, output_map).into()
    } else {
        polars::lazy::dsl::map_mul(function, exprs, output_map).into()
    }
}
