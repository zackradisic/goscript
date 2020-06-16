#![allow(dead_code)]
use super::obj::fmt_obj;
use super::objects::{DeclInfoKey, ObjKey, PackageKey, ScopeKey, TCObjects, TypeKey};
use super::operand::fmt_expr;
use super::typ::fmt_type;
use goscript_parser::ast::Expr;
use goscript_parser::objects::Objects as AstObjects;
use std::fmt::{self};

pub struct ExprDisplay<'a> {
    expr: &'a Expr,
    objs: &'a AstObjects,
}

impl<'a> ExprDisplay<'a> {
    pub fn new(expr: &'a Expr, objs: &'a AstObjects) -> ExprDisplay<'a> {
        ExprDisplay {
            expr: expr,
            objs: objs,
        }
    }
}

impl<'a> fmt::Display for ExprDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_expr(self.expr, f, self.objs)
    }
}

pub struct LangObjDisplay<'a> {
    key: &'a ObjKey,
    objs: &'a TCObjects,
}

impl<'a> LangObjDisplay<'a> {
    pub fn new(key: &'a ObjKey, objs: &'a TCObjects) -> LangObjDisplay<'a> {
        LangObjDisplay {
            key: key,
            objs: objs,
        }
    }
}

impl<'a> fmt::Display for LangObjDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_obj(self.key, f, self.objs)
    }
}

pub struct TypeDisplay<'a> {
    key: &'a TypeKey,
    objs: &'a TCObjects,
}

impl<'a> TypeDisplay<'a> {
    pub fn new(key: &'a TypeKey, objs: &'a TCObjects) -> TypeDisplay<'a> {
        TypeDisplay {
            key: key,
            objs: objs,
        }
    }
}

impl<'a> fmt::Display for TypeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_type(&Some(*self.key), f, self.objs)
    }
}
