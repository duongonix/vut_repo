//! Conditional lowering and optional guard narrowing.
use super::{Builder, Expr, Stmt, Terminator, Type};
use vut_ast::BinaryOp;

impl Builder<'_> {
    /// Narrows `name` to its optional's inner type for the current scope.
    pub(super) fn push_narrowing(&mut self, name: &str) {
        let Some(local) = self.locals.get(name).copied() else {
            return;
        };
        let Some(ty) = self.local_data[local.0].ty else {
            return;
        };
        if let Type::Optional(inner) = self.semantics.types[ty.0] {
            self.narrowed.push((name.to_string(), inner));
        }
    }
    /// Recognizes `name == null` / `name != null` on an optional local. Returns
    /// the name and whether the operator is `==` (`true`) or `!=` (`false`).
    pub(super) fn condition_narrowing(&self, condition: &Expr) -> Option<(String, bool)> {
        let Expr::Binary {
            left, op, right, ..
        } = condition
        else {
            return None;
        };
        let is_eq = match op {
            BinaryOp::Equal => true,
            BinaryOp::NotEqual => false,
            _ => return None,
        };
        let name = match (left.as_ref(), right.as_ref()) {
            (Expr::Name(name), Expr::Null(_)) | (Expr::Null(_), Expr::Name(name)) => {
                name.text.clone()
            }
            _ => return None,
        };
        let local = *self.locals.get(&name)?;
        let ty = self.local_data[local.0].ty?;
        if matches!(self.semantics.types[ty.0], Type::Optional(_)) {
            Some((name, is_eq))
        } else {
            None
        }
    }
    pub(super) fn lower_if(&mut self, statement: &vut_ast::If) {
        let narrowing = self.condition_narrowing(&statement.condition);
        let Some(condition) = self.expr(&statement.condition) else {
            return;
        };
        let then_block = self.new_block();
        let else_block = self.new_block();
        let join = self.new_block();
        self.terminate(Terminator::Branch {
            condition,
            then_block,
            else_block,
        });
        self.switch_to(then_block);
        self.conditional_depth += 1;
        let then_narrowed = self.narrowed.len();
        if let Some((name, false)) = &narrowing {
            self.push_narrowing(name);
        }
        let then_mark = self.local_data.len();
        self.lower_statements(&statement.body.statements);
        self.narrowed.truncate(then_narrowed);
        if self.is_terminated() {
            self.forget_scope(then_mark);
        } else {
            self.close_scope(then_mark);
            self.terminate(Terminator::Jump(join));
        }
        self.switch_to(else_block);
        if let Some((condition, body)) = statement.elifs.first() {
            let nested = vut_ast::If {
                condition: condition.clone(),
                body: body.clone(),
                elifs: statement.elifs[1..].to_vec(),
                otherwise: statement.otherwise.clone(),
                span: statement.span,
            };
            self.lower_if(&nested);
        } else if let Some(body) = &statement.otherwise {
            let else_narrowed = self.narrowed.len();
            if let Some((name, true)) = &narrowing {
                self.push_narrowing(name);
            }
            let else_mark = self.local_data.len();
            self.lower_statements(&body.statements);
            self.narrowed.truncate(else_narrowed);
            if self.is_terminated() {
                self.forget_scope(else_mark);
            } else {
                self.close_scope(else_mark);
            }
        }
        self.conditional_depth -= 1;
        if !self.is_terminated() {
            self.terminate(Terminator::Jump(join));
        }
        self.switch_to(join);
        // `if value == null: <terminating>` leaves `value` present afterwards.
        if let Some((name, true)) = &narrowing
            && statement.elifs.is_empty()
            && statement.otherwise.is_none()
            && block_terminates(&statement.body)
        {
            self.push_narrowing(name);
        }
    }
}

/// True when a block provably leaves the enclosing flow on every path.
fn block_terminates(block: &vut_ast::Block) -> bool {
    matches!(
        block.statements.last(),
        Some(Stmt::Return { .. } | Stmt::Break(_) | Stmt::Continue(_))
    )
}
