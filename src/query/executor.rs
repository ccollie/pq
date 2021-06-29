use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use super::aggregate_expr::AggregateExprExecutor;
use super::binary_expr::create_binary_expr_executor;
use super::function::{create_func_call_executor, FuncCallArg};
use super::identity::IdentityExecutor;
use super::parser::ast::*;
use super::samples::SampleReader;
use super::unary_expr::UnaryExprExecutor;
use super::value::{ExprValueIter, ExprValueKind};
use super::vector::VectorSelectorExecutor;
use crate::error::Result;
use crate::input::Record;
use crate::model::Timestamp;

const DEFAULT_INTERVAL: Duration = Duration::from_millis(1000);
const DEFAULT_LOOKBACK: Duration = DEFAULT_INTERVAL;

pub struct Executor {
    reader: Rc<RefCell<SampleReader>>,
    interval: Duration,
    lookback: Duration,
    start_at: Option<Timestamp>,
}

impl Executor {
    pub fn new(
        reader: Box<dyn std::iter::Iterator<Item = Record>>,
        interval: Option<Duration>,
        lookback: Option<Duration>,
        start_at: Option<Timestamp>,
    ) -> Self {
        let interval = interval.unwrap_or(DEFAULT_INTERVAL);
        assert!(interval.as_secs() + (interval.subsec_nanos() as u64) > 0);

        Self {
            reader: Rc::new(RefCell::new(SampleReader::new(reader))),
            interval,
            lookback: lookback.unwrap_or(DEFAULT_LOOKBACK),
            start_at,
        }
    }

    fn create_value_iter(&self, node: Expr) -> Box<dyn ExprValueIter> {
        match node {
            Expr::Parentheses(expr) => self.create_value_iter(*expr),

            Expr::AggregateExpr(expr) => {
                let (op, inner, modifier, parameter) = expr.into_inner();
                Box::new(AggregateExprExecutor::new(
                    op,
                    self.create_value_iter(*inner),
                    modifier,
                    parameter,
                ))
            }

            Expr::UnaryExpr(op, expr) => {
                Box::new(UnaryExprExecutor::new(op, self.create_value_iter(*expr)))
            }

            Expr::BinaryExpr(expr) => {
                let (op, lhs, rhs, bool_modifier, vector_matching, group_modifier) =
                    expr.into_inner();
                create_binary_expr_executor(
                    op,
                    self.create_value_iter(*lhs),
                    self.create_value_iter(*rhs),
                    bool_modifier,
                    vector_matching,
                    group_modifier,
                )
            }

            Expr::FunctionCall(call) => create_func_call_executor(
                call.function_name(),
                call.args()
                    .into_iter()
                    .map(|arg| match arg {
                        FunctionCallArg::Number(n) => FuncCallArg::Number(n),
                        FunctionCallArg::String(s) => FuncCallArg::String(s),
                        FunctionCallArg::Expr(expr) => {
                            FuncCallArg::ValueIter(self.create_value_iter(*expr))
                        }
                    })
                    .collect(),
            ),

            // leaf node
            Expr::NumberLiteral(val) => Box::new(IdentityExecutor::scalar(val)),

            // leaf node
            Expr::VectorSelector(sel) => Box::new(VectorSelectorExecutor::new(
                SampleReader::cursor(Rc::clone(&self.reader)),
                sel,
                self.interval,
                self.lookback,
                self.start_at,
            )),
        }
    }
}
