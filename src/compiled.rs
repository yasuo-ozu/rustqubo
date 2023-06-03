use crate::expanded::Expanded;
use crate::expr::StaticExpr;
use crate::model::Constraint;
use crate::wrapper::{Builder, Placeholder, Qubit};
use crate::{TcType, TpType, TqType};
use annealers::model::FixedSingleQuadricModel;
use annealers::node::Binary;
use annealers::variable::Real;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug)]
pub struct CompiledModel<Tp, Tq, Tc, R>
where
	Tp: TpType, // Placeholder
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	expanded: Expanded<Tp, Tq, Tc, R>,
	constraints: Vec<Constraint<Tp, Tq, Tc, R>>,
	builder: Builder<Tq>,
}

impl<Tp, Tq, Tc, R> CompiledModel<Tp, Tq, Tc, R>
where
	Tp: TpType, // Placeholder
	Tq: TqType,
	Tc: TcType,
	R: Real,
{
	pub(crate) fn new(
		expanded: Expanded<Tp, Tq, Tc, R>,
		constraints: Vec<Constraint<Tp, Tq, Tc, R>>,
	) -> Self {
		let builder = Builder::new();
		Self {
			expanded,
			constraints,
			builder,
		}
	}

	/// Feed real values to fill the placeholders.
	pub fn feed_dict(self, mut dict: HashMap<Tp, R>) -> CompiledModel<(), Tq, Tc, R> {
		let dict: HashMap<Placeholder<Tp, Tc>, R> = dict
			.drain()
			.map(|(k, v)| (Placeholder::Placeholder(k), v))
			.collect();
		let expanded = self.expanded.feed_dict(&dict).drop_placeholder();
		let constraints = self
			.constraints
			.into_iter()
			.map(|cs| cs.feed_dict(&dict).drop_placeholder())
			.collect();
		CompiledModel {
			expanded,
			constraints,
			builder: self.builder,
		}
	}

	fn generate_replace(
		set: &BTreeSet<Qubit<Tq>>,
		builder: &mut Builder<Tq>,
		p: Option<bool>,
	) -> (Expanded<Tp, Tq, Tc, R>, Option<Expanded<Tp, Tq, Tc, R>>) {
		let mut exp = Expanded::new();
		if let Some(p) = p {
			let d = set.len();
			let xs = set.iter().collect::<Vec<_>>();
			if p {
				// The following formulas are from http://www.f.waseda.jp/hfs/miru2009.pdf
				// (a * x_1 * ... * x_d) is replaced to ... (a > 0)
				let n = (d - 1) / 2;
				if d % 2 == 0 {
					// sum{i=0 -> n-1} w_i(-2 S1 + 4(i + 1) - 1)
					for i in 0..n {
						let w = builder.ancilla();
						// -2 S1 = sum{n=0 -> d-1} -2 x_i
						for j in 0..d {
							exp.insert(
								vec![w.clone(), xs[j].clone()].into_iter().collect(),
								StaticExpr::Number(R::from_i32(-2)),
							);
						}
						exp.insert(
							Some(w).into_iter().collect(),
							StaticExpr::Number(R::from_i32((4 * (i + 1) - 1) as i32)),
						);
					}
				} else {
					{
						// w_n(-S1 + 2n - 1)
						let wn = builder.ancilla();
						//
						// - S1 = sum{n=0 -> d-1} -x_i
						for j in 0..d {
							exp.insert(
								vec![wn.clone(), xs[j].clone()].into_iter().collect(),
								StaticExpr::Number(R::from_i32(-1)),
							);
						}
						exp.insert(
							Some(wn).into_iter().collect(),
							StaticExpr::Number(R::from_i32((2 * n - 1) as i32)),
						);
					}
					// sum{i=0 -> n-2} w_i(-2S1 + 4(i + 1) - 1)
					for i in 0..n - 1 {
						let w = builder.ancilla();
						// -2 S1 = sum{n=0 -> d-1} -2 x_i
						for j in 0..d {
							exp.insert(
								vec![w.clone(), xs[j].clone()].into_iter().collect(),
								StaticExpr::Number(R::from_i32(-2)),
							);
						}
						exp.insert(
							Some(w).into_iter().collect(),
							StaticExpr::Number(R::from_i32(4 * (i as i32 + 1) - 1)),
						);
					}
				}
				// sum{i=0 -> d-2} sum{j=i+1 -> d-1} x_ix_j
				for i in 0..d {
					for j in i + 1..d {
						exp.insert(
							vec![xs[i].clone(), xs[j].clone()].into_iter().collect(),
							StaticExpr::Number(R::from_i32(1)),
						);
					}
				}
			} else {
				// a * x_1 * ... * x_d = min a * w  * { x_1 * ... * x_d - (d - 1) }  (a < 0)
				let w = builder.ancilla();
				for x in set.iter() {
					exp.insert(
						vec![w.clone(), x.clone()].into_iter().collect(),
						StaticExpr::Number(R::from_i32(1)),
					);
				}
				exp.insert(
					Some(w).into_iter().collect(),
					StaticExpr::Number(R::from_i32(1 - d as i32)),
				);
			}
			(exp, None)
		} else {
			// Cannot determine sign of a
			// x * y -> min{1 + w * (3 - 2x - 2y)}, xyz = a * w
			if let &[x, y] = &set.iter().take(2).collect::<Vec<&Qubit<Tq>>>() as &[&Qubit<Tq>] {
				let w = builder.ancilla();
				exp.insert(
					Some(w.clone()).into_iter().collect(),
					StaticExpr::Number(R::from_i32(3)),
				);
				exp.insert(
					vec![x, &w].into_iter().cloned().collect(),
					StaticExpr::Number(R::from_i32(-2)),
				);
				exp.insert(
					(vec![y, &w]).into_iter().cloned().collect(),
					StaticExpr::Number(R::from_i32(-2)),
				);
				exp.insert(
					(vec![x, y]).into_iter().cloned().collect(),
					StaticExpr::Number(R::from_i32(1)),
				);
				(Expanded::from_qubit(w), Some(exp))
			} else {
				panic!();
			}
		}
	}

	pub(crate) fn get_unsatisfied_constraints(
		&self,
		map: &HashMap<&Qubit<Tq>, bool>,
	) -> Vec<&Constraint<Tp, Tq, Tc, R>> {
		self.constraints
			.iter()
			.filter(|cc| !cc.is_satisfied(map))
			.collect()
	}

	pub(crate) fn reduce_order(mut self, max_order: usize) -> Self {
		let mut builder = self.builder.clone();
		while self.expanded.get_order() > max_order {
			let mut m = self.expanded.count_qubit_subsets(max_order, 2, None);
			if let Some(max_count) = m.values().map(|nonzero| (*nonzero).get()).max() {
				let sets = m
					.drain()
					.filter_map(|(k, v)| if v.get() == max_count { Some(k) } else { None })
					.collect::<Vec<_>>();
				let max_set_size = sets.iter().map(|(set, _)| set.len()).max().unwrap();
				let (replaced_set, p) = sets
					.into_iter()
					.filter(|(set, _)| set.len() == max_set_size)
					.next()
					.unwrap();
				let replaced_set = replaced_set.into_iter().cloned().collect();
				let (replacing_exp, constraint) =
					Self::generate_replace(&replaced_set, &mut builder, p);
				let mut new_expanded = Expanded::new();
				for mut expanded in self
					.expanded
					.drain()
					.map(|(set, exp)| Expanded::from(set, exp))
				{
					if expanded.is_superset(&replaced_set) {
						expanded = expanded.remove_qubits(&replaced_set);
						expanded *= replacing_exp.clone();
					}
					new_expanded += expanded;
				}
				self.expanded = new_expanded;
				if let Some(constraint) = constraint {
					self.constraints
						.push(Constraint::from_raw(None, constraint.into(), None));
				}
			} else {
				break;
			}
		}
		self.builder = builder;
		self
	}

	pub(crate) fn get_qubits(&self) -> BTreeSet<&Qubit<Tq>> {
		self.expanded.get_qubits()
	}

	pub fn get_placeholders(&self) -> BTreeSet<&Placeholder<Tp, Tc>> {
		self.expanded.get_placeholders()
	}

	// TODO: support HashMap-based model
	pub(crate) fn generate_qubo<F>(
		&self,
		qubits: &[&Qubit<Tq>],
		ph_feedback: &mut F,
	) -> (R, FixedSingleQuadricModel<Binary<R>>)
	where
		F: FnMut(&Placeholder<Tp, Tc>) -> R,
	{
		self.expanded.generate_qubo(qubits, ph_feedback)
	}
}
