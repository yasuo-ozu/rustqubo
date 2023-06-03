use crate::expr::{Expr, StaticExpr};
use crate::wrapper::{Placeholder, Qubit};
use crate::{TcType, TpType, TqType};
use annealers::model::FixedSingleQuadricModel;
use annealers::node::Binary;
use annealers::variable::Real;
use std::collections::{BTreeSet, HashMap};
use std::convert::From;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::ops::{Add, AddAssign, Mul, MulAssign};

#[test]
fn get_subsets_test() {
	let set: BTreeSet<i32> = vec![1, 2, 3, 4].into_iter().collect();
	let mut i = 0;
	get_subsets(&set, 2, None, move |set| match i {
		0 => {
			assert_eq!(set, &vec![&3, &4].into_iter().collect());
			i += 1;
		}
		1 => {
			assert_eq!(set, &vec![&2, &4].into_iter().collect());
			i += 1;
		}
		2 => {
			assert_eq!(set, &vec![&2, &3].into_iter().collect());
			i += 1;
		}
		3 => {
			assert_eq!(set, &vec![&2, &3, &4].into_iter().collect());
			i += 1;
		}
		4 => {
			assert_eq!(set, &vec![&1, &4].into_iter().collect());
			i += 1;
		}
		5 => {
			assert_eq!(set, &vec![&1, &3].into_iter().collect());
			i += 1;
		}
		6 => {
			assert_eq!(set, &vec![&1, &3, &4].into_iter().collect());
			i += 1;
		}
		7 => {
			assert_eq!(set, &vec![&1, &2].into_iter().collect());
			i += 1;
		}
		8 => {
			assert_eq!(set, &vec![&1, &2, &4].into_iter().collect());
			i += 1;
		}
		9 => {
			assert_eq!(set, &vec![&1, &2, &3].into_iter().collect());
			i += 1;
		}
		10 => {
			assert_eq!(set, &vec![&1, &2, &3, &4].into_iter().collect());
			i += 1;
		}
		_ => panic!("{:?}", set),
	})
}

fn get_subsets<'a, T, F>(set: &'a BTreeSet<T>, min: usize, max: Option<usize>, mut cb: F)
where
	F: FnMut(&BTreeSet<&'a T>) -> (),
	T: Eq + Hash + Ord,
{
	let set: Vec<&T> = set.iter().collect();
	let max = std::cmp::min(max.unwrap_or(usize::max_value()), set.len());
	fn internal<'a, T, F>(
		set: &Vec<&'a T>,
		inner: &mut BTreeSet<&'a T>,
		loc: usize,
		min: usize,
		max: usize,
		cb: &mut F,
	) where
		F: FnMut(&BTreeSet<&'a T>) -> (),
		T: Eq + Hash + Ord,
	{
		if loc == set.len() {
			cb(inner);
		} else {
			if set.len() - loc - 1 + inner.len() >= min {
				internal(set, inner, loc + 1, min, max, cb);
			}
			if inner.len() < max {
				let item: &T = set[loc];
				inner.insert(item);
				internal(set, inner, loc + 1, min, max, cb);
				inner.remove(item);
			}
		}
	}
	let mut sub = BTreeSet::new();
	internal(&set, &mut sub, 0, min, max, &mut cb);
}

#[derive(Default, Clone, Debug)]
pub(crate) struct Expanded<Tp, Tq, Tc, R>(
	HashMap<BTreeSet<Qubit<Tq>>, StaticExpr<Placeholder<Tp, Tc>, R>>,
)
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real;

impl<Tp, Tq, Tc, R> Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn from(set: BTreeSet<Qubit<Tq>>, exp: StaticExpr<Placeholder<Tp, Tc>, R>) -> Self {
		let mut m = HashMap::new();
		m.insert(set, exp);
		Self(m)
	}

	pub fn from_qubit(q: Qubit<Tq>) -> Self {
		let mut m = HashMap::new();
		m.insert(
			Some(q).into_iter().collect(),
			StaticExpr::Number(R::from_i32(1)),
		);
		Self(m)
	}

	pub(crate) fn drop_placeholder(mut self) -> Expanded<(), Tq, Tc, R> {
		Expanded(
			self.0
				.drain()
				.map(|(key, exp)| (key, exp.drop_placeholder()))
				.collect(),
		)
	}

	pub fn feed_dict(mut self, dict: &HashMap<Placeholder<Tp, Tc>, R>) -> Self {
		Self(
			self.0
				.drain()
				.map(|(key, exp)| (key, exp.feed_dict(dict)))
				.collect(),
		)
	}

	pub fn is_superset(&self, other: &BTreeSet<Qubit<Tq>>) -> bool {
		self.0.iter().all(|(set, _)| set.is_superset(other))
	}

	pub fn get_order(&self) -> usize {
		self.0.iter().map(|(set, _)| set.len()).max().unwrap_or(0)
	}

	pub fn remove_qubits(self, qubits: &BTreeSet<Qubit<Tq>>) -> Self {
		Self(
			self.0
				.into_iter()
				.map(|(mut set, exp)| {
					for q in qubits.iter() {
						set.remove(q);
					}
					(set, exp)
				})
				.collect(),
		)
	}

	pub fn get_placeholders(&self) -> BTreeSet<&Placeholder<Tp, Tc>> {
		let mut ret = BTreeSet::new();
		for (_, exp) in self.0.iter() {
			ret = ret
				.union(&exp.get_placeholders().into_iter().collect())
				.cloned()
				.collect()
		}
		ret
	}

	pub fn get_qubits(&self) -> BTreeSet<&Qubit<Tq>> {
		let mut ret = BTreeSet::new();
		for (qubits, _) in self.0.iter() {
			ret = ret.union(&qubits.iter().collect()).cloned().collect()
		}
		ret
	}

	pub fn generate_qubo<F>(
		&self,
		qubits: &[&Qubit<Tq>],
		ph_feedback: &mut F,
	) -> (R, FixedSingleQuadricModel<Binary<R>>)
	where
		F: FnMut(&Placeholder<Tp, Tc>) -> R,
	{
		let dict = qubits
			.iter()
			.cloned()
			.enumerate()
			.map(|(i, q)| (q, i))
			.collect::<HashMap<&Qubit<Tq>, usize>>();
		let mut c = R::from_i32(0);
		let mut model = FixedSingleQuadricModel::new(Binary::new(), qubits.len());
		for (set, expr) in self.0.iter() {
			let val = expr.calculate(ph_feedback);
			match &set.iter().collect::<Vec<_>>() as &[&Qubit<Tq>] {
				&[] => c += val,
				&[q] => {
					if let Some(index) = dict.get(q) {
						model.add_weight(*index, *index, val);
					} else {
						panic!()
					}
				}
				&[q1, q2] => {
					if let (Some(index1), Some(index2)) = (dict.get(q1), dict.get(q2)) {
						model.add_weight(*index1, *index2, val);
					} else {
						panic!()
					}
				}
				_ => panic!("Cannot make qubo"),
			}
		}
		(c, model)
	}

	pub fn count_qubit_subsets(
		&self,
		max_order: usize,
		min: usize,
		max: Option<usize>,
	) -> HashMap<(BTreeSet<&Qubit<Tq>>, Option<bool>), NonZeroUsize> {
		let mut m: HashMap<(BTreeSet<&Qubit<Tq>>, Option<bool>), NonZeroUsize> = HashMap::new();
		for (sup, expr) in self.0.iter() {
			if sup.len() <= max_order {
				continue;
			}
			get_subsets(sup, min, max, |sub| {
				let expr_info = if sub.len() > 2 {
					if let Some(b) = expr.is_positive() {
						Some(b)
					} else {
						return;
					}
				} else {
					None
				};
				if let Some(v) = m.get_mut(&(sub.clone(), expr_info)) {
					let i = v.get();
					assert!(i < usize::max_value());
					unsafe {
						*v = NonZeroUsize::new_unchecked(i + 1);
					}
				} else {
					unsafe {
						m.insert((sub.clone(), expr_info), NonZeroUsize::new_unchecked(1));
					}
				}
			})
		}
		m
	}
}

impl<Tp, Tq, Tc, R> std::ops::Deref for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	type Target = HashMap<BTreeSet<Qubit<Tq>>, StaticExpr<Placeholder<Tp, Tc>, R>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<Tp, Tq, Tc, R> std::ops::DerefMut for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<Tp, Tq, Tc, R> Into<Expr<Placeholder<Tp, Tc>, Qubit<Tq>, Tc, R>> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
	Expr<Placeholder<Tp, Tc>, Qubit<Tq>, Tc, R>: Mul<
		Expr<Placeholder<Tp, Tc>, Qubit<Tq>, Tc, R>,
		Output = Expr<Placeholder<Tp, Tc>, Qubit<Tq>, Tc, R>,
	>,
{
	fn into(self) -> Expr<Placeholder<Tp, Tc>, Qubit<Tq>, Tc, R> {
		let mut expr = None;
		for (set, sexp) in self.0.into_iter() {
			let e = set
				.into_iter()
				.fold(sexp.into(), |expr, q| expr * Expr::Binary(q));
			if let Some(ee) = expr {
				expr = Some(Expr::Add(Box::new(e), Box::new(ee)))
			} else {
				expr = Some(e);
			}
		}
		expr.unwrap_or(Expr::Number(R::from_i32(0)))
	}
}

impl<Tp, Tq, Tc, R> From<Tq> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	fn from(lb: Tq) -> Self {
		Expanded(
			Some((
				Some(Qubit::new(lb)).into_iter().collect(),
				StaticExpr::Number(R::from_i32(1)),
			))
			.into_iter()
			.collect(),
		)
	}
}

impl<Tp, Tq, Tc, R> From<StaticExpr<Placeholder<Tp, Tc>, R>> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	fn from(e: StaticExpr<Placeholder<Tp, Tc>, R>) -> Self {
		let mut ret = HashMap::new();
		ret.insert(None.into_iter().collect(), e);
		Expanded(ret)
	}
}

impl<Tp, Tq, Tc, R, RHS> AddAssign<RHS> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
	RHS: Into<Self>,
{
	#[inline]
	fn add_assign(&mut self, other: RHS) {
		let other = other.into();
		for (k, v) in other.0.into_iter() {
			if let Some(e) = self.0.remove(&k) {
				self.0.insert(k, StaticExpr::Add(vec![e, v]).simplify());
			} else {
				self.0.insert(k, v);
			}
		}
	}
}
impl<Tp, Tq, Tc, R, RHS> Add<RHS> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
	RHS: Into<Self>,
{
	type Output = Self;
	#[inline]
	fn add(mut self, other: RHS) -> Self::Output {
		self.add_assign(other);
		self
	}
}

impl<Tp, Tq, Tc, R, RHS> MulAssign<RHS> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
	RHS: Into<Self>,
{
	#[inline]
	fn mul_assign(&mut self, other: RHS) {
		let other = other.into();
		let it = self.0.iter().flat_map(|(k1, v1)| {
			other.0.iter().map(move |(k2, v2)| {
				(
					k1.iter().chain(k2.iter()).cloned().collect(),
					StaticExpr::Mul(vec![v1.clone(), v2.clone()]).simplify(),
				)
			})
		});
		let mut m = HashMap::new();
		for (k, v) in it {
			if let Some(e) = m.remove(&k) {
				m.insert(k, StaticExpr::Add(vec![e, v]).simplify());
			} else {
				m.insert(k, v);
			}
		}
		std::mem::swap(&mut m, &mut self.0);
	}
}

impl<Tp, Tq, Tc, R, RHS> Mul<RHS> for Expanded<Tp, Tq, Tc, R>
where
	Tp: TpType,
	Tq: TqType,
	Tc: TcType,
	R: Real,
	RHS: Into<Self>,
{
	type Output = Self;
	#[inline]
	fn mul(mut self, other: RHS) -> Self::Output {
		self.mul_assign(other);
		self
	}
}
