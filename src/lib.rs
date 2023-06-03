//! RustQUBO is a powerful library to create QUBO from polynomial expressions
//! with constraints and placeholders.
//!
//! # Examples
//!
//! ## Simple example
//! ```
//! # extern crate rustqubo;
//! # use rustqubo::Expr;
//! # use rustqubo::solve::SimpleSolver;
//! let hmlt = -Expr::Spin("a") * Expr::Spin("b") * Expr::Number(2) + Expr::Spin("a") * Expr::Number(3);
//! let compiled = hmlt.compile();
//! let solver = SimpleSolver::new(&compiled);
//! let (c, qubits) = solver.solve().unwrap();
//! // displays -5.0, {"a": false, "b": false}
//! println!("{}, {:?}", &c, &qubits);
//! # assert_eq!(qubits.get(&"a"), Some(false));
//! # assert_eq!(qubits.get(&"b"), Some(false));
//! # assert_eq!(c, -5);
//! ```
//!
//! ## Example with constraints
//! ```
//! # extern crate rustqubo;
//! # use rustqubo::Expr;
//! # use rustqubo::solve::SimpleSolver;
//! let hmlt = Expr::Constraint{
//! 		label: "constraint1",
//! 		expr: Box::new((Expr::Binary(0) + Expr::Binary(1) - Expr::Number(1)) ^ 2usize)
//! 	} + Expr::Binary(0) * Expr::Number(30);
//! let compiled = hmlt.compile();
//! let solver = SimpleSolver::new(&compiled);
//! let (c, qubits, unsatisfied) = solver.solve_with_constraints().unwrap();
//! // displays 0, {0: false, 1: true}, []
//! println!("{}, {:?}, {:?}", &c, &qubits, &unsatisfied);
//! # assert_eq!(c, 0);
//! # assert_eq!(qubits.get(&0), Some(false));
//! # assert_eq!(qubits.get(&1), Some(true));
//! # assert_eq!(unsatisfied.len(), 0);
//! ```
use std::cmp::Ord;
use std::fmt::Debug;
use std::hash::Hash;

extern crate annealers;
extern crate rand;
extern crate rayon;

#[cfg(feature = "pyo3")]
extern crate pyo3;

pub trait LabelType: PartialEq + Eq + Clone + std::fmt::Debug {}
pub trait TpType: LabelType + Hash + Ord {}
pub trait TqType: LabelType + Hash + Ord {}
pub trait TcType: LabelType + Hash + Ord {}

impl<T> LabelType for T where T: PartialEq + Eq + Clone + Debug {}
impl<T> TpType for T where T: LabelType + Hash + Ord {}
impl<T> TqType for T where T: LabelType + Hash + Ord {}
impl<T> TcType for T where T: LabelType + Hash + Ord {}

// mod anneal;
mod compiled;
mod expanded;
mod expr;
mod model;
pub mod solution;
pub mod solve;
mod util;
mod wrapper;

#[cfg(feature = "python")]
pub mod python;

pub use expr::Expr;

#[test]
fn expr_test() {
	let _: Expr<(), _, (), i32> = 2i32 * Expr::Binary(("a", "b")) * 3i32;
}
