#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

use std::collections::HashMap;
use std::convert::identity;
use std::fmt;
use std::hash::Hash;
use std::ops::Neg;

#[derive(Clone, Debug)]
pub struct Variable {
    name: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct VariableRef(u32);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Literal {
    pub variable: VariableRef,
    pub is_positive: bool,
}

impl Literal {
    fn evaluate(&self, assignment: &Assignment) -> bool {
        assignment.values.get(&self.variable).map(|&value| value ^ !self.is_positive).unwrap_or(false)
    }
}

impl Neg for Literal {
    type Output = Literal;

    fn neg(self) -> Self::Output {
        Literal {
            is_positive: !self.is_positive,
            ..self
        }
    }
}

#[derive(Clone, Debug)]
pub struct Clause {
    pub literals: Vec<Literal>,
}

impl Clause {
    fn evaluate(&self, assignment: &Assignment) -> bool {
        self.literals.iter().map(|literal| literal.evaluate(assignment)).reduce(|a, b| a || b).unwrap_or(false)
    }
}

#[derive(Clone)]
pub struct Formula {
    pub clauses: Vec<Clause>,
}

impl Formula {
    pub fn evaluate(&self, assignment: &Assignment) -> bool {
        self.clauses.iter().map(|clause| clause.evaluate(assignment)).reduce(|a, b| a && b).unwrap_or(true)
    }

    fn first_pure_literal(&self) -> Option<Literal> {
        let mut variable_map: HashMap<VariableRef, Option<Literal>> = HashMap::new();
        for clause in &self.clauses {
            for literal in &clause.literals {
                let variable = literal.variable;
                if let Some(prev_literal) = variable_map.get_mut(&variable) {
                    if let Some(prev_literal_value) = prev_literal {
                        if literal.is_positive != prev_literal_value.is_positive {
                            *prev_literal = None;
                        }
                    }
                } else {
                    variable_map.insert(variable, Some(*literal));
                }
            }
        }

        variable_map.values().copied().find_map(identity)
    }

    fn first_unassigned_variable(&self, assignment: &Assignment) -> Option<VariableRef> {
        self.clauses.iter().find_map(|clause| clause.literals.iter().find_map(|literal| {
            let variable = literal.variable;
            if !assignment.values.contains_key(&variable) {
                Some(variable)
            } else {
                None
            }
        }))
    }

    fn first_unit_clause_literal(&self) -> Option<Literal> {
        for clause in &self.clauses {
            if clause.literals.len() == 1 {
                return Some(clause.literals[0]);
            }
        }

        None
    }
}

#[derive(Clone)]
pub struct Instance {
    pub variables: Vec<Variable>,
    pub formula: Formula,
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        writeln!(f)?;
        for clause in &self.formula.clauses {
            write!(f, "/\\ (")?;
            for (i, literal) in clause.literals.iter().enumerate() {
                if i != 0 {
                    write!(f, " \\/ ")?;
                }
                let variable_name = &self.variables[literal.variable.0 as usize].name;
                if !literal.is_positive {
                    write!(f, "-")?;
                }
                write!(f, "{}", variable_name)?;
            }
            writeln!(f, ")")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Assignment {
    pub values: HashMap<VariableRef, bool>,
}

impl Assignment {
    fn empty() -> Assignment {
        Assignment {
            values: HashMap::new(),
        }
    }

    fn insert_assignment(&self, variable: VariableRef, value: bool) -> Assignment {
        let mut ret = self.clone();
        ret.values.insert(variable, value);
        ret
    }

    fn insert_satisfying_assignment(&self, literal: Literal) -> Assignment {
        let mut ret = self.clone();
        ret.values.insert(literal.variable, literal.is_positive);
        ret
    }
}

#[derive(Debug)]
pub enum Satisfiability {
    Satisfiable(Assignment),
    Unsatisfiable,
}

impl Satisfiability {
    fn is_satisfiable(&self) -> bool {
        match self {
            Satisfiability::Satisfiable(_) => true,
            Satisfiability::Unsatisfiable => false,
        }
    }
}

pub fn backtracking(formula: &Formula) -> Satisfiability {
    fn go(formula: &Formula, assignment: Assignment) -> Satisfiability {
        if formula.evaluate(&assignment) {
            return Satisfiability::Satisfiable(assignment);
        }
        if let Some(variable) = formula.first_unassigned_variable(&assignment) {
            let positive_assignment = assignment.insert_assignment(variable, true);
            let result = go(formula, positive_assignment);
            if result.is_satisfiable() {
                return result;
            }
            let negative_assignment = assignment.insert_assignment(variable, false);
            let result = go(formula, negative_assignment);
            if result.is_satisfiable() {
                return result;
            }
        }
        Satisfiability::Unsatisfiable
    }
    go(formula, Assignment::empty())
}

pub fn dpll(formula: &Formula) -> Satisfiability {
    fn go(formula: &Formula, assignment: Assignment) -> Satisfiability {
        if formula.clauses.is_empty() {
            return Satisfiability::Satisfiable(assignment);
        }

        // Unit clause rule
        if let Some(literal) = formula.first_unit_clause_literal() {
            let clauses = formula.clauses.iter().filter_map(|clause| {
                if clause.literals.contains(&literal) {
                    return None;
                }

                let literals = clause.literals.iter().copied().filter(|&l| l != -literal).collect::<Vec<_>>();
                Some(Clause {
                    literals,
                })
            }).collect();
            let assignment = assignment.insert_satisfying_assignment(literal);
            return go(&Formula {
                clauses,
            }, assignment);
        }

        // Pure literal rule
        if let Some(literal) = formula.first_pure_literal() {
            let clauses = formula.clauses.iter().filter(|clause| {
                !clause.literals.contains(&literal) && !clause.literals.contains(&-literal)
            }).cloned().collect();
            let assignment = assignment.insert_satisfying_assignment(literal);
            return go(&Formula {
                clauses,
            }, assignment);
        }

        // Splitting rule
        if let Some(variable) = formula.first_unassigned_variable(&assignment) {
            //  Positive case
            let clauses = formula.clauses.iter().filter_map(|clause| {
                if clause.literals.contains(&Literal {
                    variable,
                    is_positive: true,
                }) {
                    return None;
                }

                Some(Clause {
                    literals: clause.literals.iter().copied().filter(|&literal| literal != Literal {
                        variable,
                        is_positive: false,
                    }).collect(),
                })
            }).collect();
            let positive_assignment = assignment.insert_assignment(variable, true);
            let result = go(&Formula {
                clauses,
            }, positive_assignment);
            if result.is_satisfiable() {
                return result;
            }
            //  Negative case
            let clauses = formula.clauses.iter().filter_map(|clause| {
                if clause.literals.contains(&Literal {
                    variable,
                    is_positive: false,
                }) {
                    return None;
                }

                Some(Clause {
                    literals: clause.literals.iter().copied().filter(|&literal| literal != Literal {
                        variable,
                        is_positive: true,
                    }).collect(),
                })
            }).collect();
            let negative_assignment = assignment.insert_assignment(variable, false);
            let result = go(&Formula {
                clauses,
            }, negative_assignment);
            if result.is_satisfiable() {
                return result;
            }
        }

        Satisfiability::Unsatisfiable
    }
    go(formula, Assignment::empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate quickcheck;

    const ARBITRARY_NUM_VARIABLES: u32 = 8;

    impl quickcheck::Arbitrary for VariableRef {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            VariableRef(u32::arbitrary(g) % ARBITRARY_NUM_VARIABLES)
        }
    }

    impl quickcheck::Arbitrary for Literal {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Literal {
                variable: VariableRef::arbitrary(g),
                is_positive: bool::arbitrary(g),
            }
        }
    }

    impl quickcheck::Arbitrary for Clause {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let max_num_literals = 4;
            // TODO: This excludes 0, which typically results in more interesting instances,
            //  since any empty clause renders the entire instance unsatisfiable. However,
            //  this may be an important case to check, so we may want to come up with a way
            //  to conditionally include empty clauses with low probability of occurrence.
            let num_literals = (u32::arbitrary(g) % max_num_literals) + 1;
            Clause {
                literals: (0..num_literals).map(|_| Literal::arbitrary(g)).collect(),
            }
        }
    }

    impl quickcheck::Arbitrary for Instance {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let variables = (0..ARBITRARY_NUM_VARIABLES).map(|i| Variable {
                name: format!("v{}", i),
            }).collect();

            // TODO: Find a good way to respect size that doesn't end up generating too many
            //  unsatisfiable instances
            let num_clauses = 4;//g.size();
            let formula = Formula {
                clauses: (0..num_clauses).map(|_| Clause::arbitrary(g)).collect(),
            };

            Instance {
                variables,
                formula,
            }
        }
    }

    #[quickcheck]
    fn backtracking_satisfying_assignments_are_satisfying(instance: Instance) -> bool {
        match backtracking(&instance.formula) {
            Satisfiability::Satisfiable(assignment) => {
                println!("Satisfying assignment: {:?}", assignment);
                instance.formula.evaluate(&assignment)
            }
            Satisfiability::Unsatisfiable => true,
        }
    }

    #[quickcheck]
    fn dpll_satisfying_assignments_are_satisfying(instance: Instance) -> bool {
        match dpll(&instance.formula) {
            Satisfiability::Satisfiable(assignment) => {
                println!("Satisfying assignment: {:?}", assignment);
                instance.formula.evaluate(&assignment)
            }
            Satisfiability::Unsatisfiable => true,
        }
    }

    #[quickcheck]
    fn backtracking_and_dpll_reach_the_same_conclusion(instance: Instance) -> bool {
        let backtracking_result = backtracking(&instance.formula);
        println!("backtracking result: {:?}", backtracking_result);
        let dpll_result = dpll(&instance.formula);
        println!("dpll result: {:?}", dpll_result);
        let ret = backtracking_result.is_satisfiable() == dpll_result.is_satisfiable();
        println!();
        ret
    }
}
