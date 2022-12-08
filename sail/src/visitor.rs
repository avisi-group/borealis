//! Visitor pattern for Sail AST
//!
//! Vistitor trait has overridable methods

use crate::{
    ast::{
        AliasSpec, Ast, BaseEffect, Comment, CommentRoot, DecSpec, DefaultSpec, Definition, Effect,
        EffectOpt, Expression, FieldExpression, FunctionClause, FunctionDefinition, Identifier,
        IndexRange, InternalLoopMeasure, KindIdentifier, KindedIdentifier, LValueExpression,
        LetBind, Literal, LoopMeasure, MappingClause, MappingDefinition, MappingPattern,
        MappingPatternExpression, NConstraint, NumericExpression, Order, Pattern, PatternMatch,
        QuantItem, RecursiveAnnotationOpt, RegisterId, ScatteredDefinition, Typ, TypArg, TypPat,
        TypQuant, TypeAnnotationOpt, TypeDefinition, TypeScheme, TypeUnion, Value,
        ValueSpecification,
    },
    num::BigInt,
    types::EnumWrapper,
};

/// Visitor trait for interacting with Sail AST
pub trait Visitor: Sized {
    #[allow(missing_docs)]
    fn visit_root(&mut self, node: &Ast) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_definition(&mut self, node: &EnumWrapper<Definition>) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_type_definition(&mut self, node: &TypeDefinition) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_identifier(&mut self, node: &Identifier) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_comment_root(&mut self, node: &CommentRoot) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_comment(&mut self, node: &Comment) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_typquant(&mut self, node: &TypQuant) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_typarg(&mut self, node: &TypArg) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_quantitem(&mut self, node: &QuantItem) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_numeric_expression(&mut self, node: &NumericExpression) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_typ(&mut self, node: &Typ) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_order(&mut self, node: &Order) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_nconstraint(&mut self, node: &NConstraint) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_kinded_identifier(&mut self, node: &KindedIdentifier) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_kind_identifier(&mut self, node: &KindIdentifier) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_big_int(&mut self, node: &BigInt) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_function_definition(&mut self, node: &FunctionDefinition) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_recursive_annotation_opt(&mut self, node: &RecursiveAnnotationOpt) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_type_annotation_opt(&mut self, node: &TypeAnnotationOpt) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_effect_opt(&mut self, node: &EffectOpt) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_function_clause(&mut self, node: &FunctionClause) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_default_spec(&mut self, node: &DefaultSpec) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_value_specification(&mut self, node: &ValueSpecification) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_type_scheme(&mut self, node: &TypeScheme) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_effect(&mut self, node: &Effect) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_base_effect(&mut self, node: &BaseEffect) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_pattern(&mut self, node: &Pattern) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_expression(&mut self, node: &Expression) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_literal(&mut self, node: &Literal) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_typpat(&mut self, node: &TypPat) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_pattern_match(&mut self, node: &PatternMatch) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_internal_loop_measure(&mut self, node: &InternalLoopMeasure) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_field_expression(&mut self, node: &FieldExpression) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_letbind(&mut self, node: &LetBind) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_lvalue_expression(&mut self, node: &LValueExpression) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_value(&mut self, node: &EnumWrapper<Value>) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_typunion(&mut self, node: &TypeUnion) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_indexrange(&mut self, node: &IndexRange) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_decspec(&mut self, node: &DecSpec) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_aliasspec(&mut self, node: &AliasSpec) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_registerid(&mut self, node: &RegisterId) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_mapping_definition(&mut self, node: &MappingDefinition) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_scattered_definition(&mut self, node: &ScatteredDefinition) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_loop_measure(&mut self, node: &LoopMeasure) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_mapping_clause(&mut self, node: &MappingClause) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_mapping_pattern_expression(&mut self, node: &MappingPatternExpression) {
        node.walk(self);
    }

    #[allow(missing_docs)]
    fn visit_mapping_pattern(&mut self, node: &MappingPattern) {
        node.walk(self);
    }
}

/// Trait encapsulating the traversing logic for the AST
pub trait Walkable {
    /// Visit children of the current node
    fn walk<V: Visitor>(&self, visitor: &mut V);
}
