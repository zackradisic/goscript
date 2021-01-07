#![allow(dead_code)]
use goscript_parser::ast::Node;
use goscript_parser::ast::{Expr, NodeId};
use goscript_parser::objects::IdentKey;
use goscript_types::{
    BasicType, ConstValue, EntityType, ObjKey, OperandMode, PackageKey as TCPackageKey, TCObjects,
    Type, TypeInfo, TypeKey as TCTypeKey,
};
use goscript_vm::instruction::{OpIndex, ValueType};
use goscript_vm::metadata::*;
use goscript_vm::value::*;
use std::collections::HashMap;

pub type TypeCache = HashMap<TCTypeKey, GosMetadata>;

pub struct TypeLookup<'a> {
    tc_objs: &'a TCObjects,
    ti: &'a TypeInfo,
    types_cache: &'a mut TypeCache,
}

impl<'a> TypeLookup<'a> {
    pub fn new(
        tc_objs: &'a TCObjects,
        ti: &'a TypeInfo,
        cache: &'a mut TypeCache,
    ) -> TypeLookup<'a> {
        TypeLookup {
            tc_objs: tc_objs,
            ti: ti,
            types_cache: cache,
        }
    }

    pub fn type_info(&self) -> &TypeInfo {
        self.ti
    }

    pub fn get_tc_const_value(&mut self, id: NodeId) -> Option<&ConstValue> {
        let typ_val = self.ti.types.get(&id).unwrap();
        typ_val.get_const_val()
    }

    pub fn get_const_value(&mut self, id: NodeId) -> GosValue {
        let typ_val = self.ti.types.get(&id).unwrap();
        let const_val = typ_val.get_const_val().unwrap();
        self.const_value(typ_val.typ, const_val)
    }

    pub fn get_expr_tc_type(&self, e: &Expr) -> TCTypeKey {
        self.get_node_tc_type(e.id())
    }

    pub fn get_node_tc_type(&self, id: NodeId) -> TCTypeKey {
        self.ti.types.get(&id).unwrap().typ
    }

    pub fn get_expr_mode(&self, e: &Expr) -> &OperandMode {
        &self.ti.types.get(&e.id()).unwrap().mode
    }

    // some of the built in funcs are not recorded
    pub fn try_get_expr_tc_type(&self, e: &Expr) -> Option<TCTypeKey> {
        self.ti
            .types
            .get(&e.id())
            .map(|x| {
                let typ = x.typ;
                if self.tc_objs.types[typ].is_invalid(self.tc_objs) {
                    None
                } else {
                    Some(typ)
                }
            })
            .flatten()
    }

    pub fn try_get_pkg_key(&self, e: &Expr) -> Option<TCPackageKey> {
        if let Expr::Ident(ikey) = e {
            self.ti
                .uses
                .get(ikey)
                .map(|x| {
                    if let EntityType::PkgName(pkg, _) = self.tc_objs.lobjs[*x].entity_type() {
                        Some(*pkg)
                    } else {
                        None
                    }
                })
                .flatten()
        } else {
            None
        }
    }

    pub fn get_expr_value_type(&mut self, e: &Expr) -> ValueType {
        let tv = self.ti.types.get(&e.id()).unwrap();
        if tv.mode == OperandMode::TypeExpr {
            ValueType::Metadata
        } else {
            self.value_type_from_tc(self.get_expr_tc_type(e))
        }
    }

    pub fn get_meta_by_node_id(&mut self, id: NodeId, objects: &mut VMObjects) -> GosMetadata {
        let tv = self.ti.types.get(&id).unwrap();
        let mut md = self.meta_from_tc(tv.typ, objects);
        md.set_is_type(tv.mode == OperandMode::TypeExpr);
        md
    }

    pub fn get_use_tc_type(&self, ikey: IdentKey) -> TCTypeKey {
        let obj = &self.tc_objs.lobjs[self.ti.uses[&ikey]];
        obj.typ().unwrap()
    }

    pub fn get_use_value_type(&self, ikey: IdentKey) -> ValueType {
        self.value_type_from_tc(self.get_use_tc_type(ikey))
    }

    pub fn get_def_tc_type(&self, ikey: IdentKey) -> TCTypeKey {
        let obj = &self.tc_objs.lobjs[self.ti.defs[&ikey].unwrap()];
        obj.typ().unwrap()
    }

    pub fn get_def_value_type(&mut self, ikey: IdentKey) -> ValueType {
        self.value_type_from_tc(self.get_def_tc_type(ikey))
    }

    pub fn gen_def_type_meta(&mut self, ikey: IdentKey, objects: &mut VMObjects) -> GosMetadata {
        self.meta_from_tc(self.get_def_tc_type(ikey), objects)
    }

    pub fn get_range_tc_types(&mut self, e: &Expr) -> [TCTypeKey; 3] {
        let typ = self.ti.types.get(&e.id()).unwrap().typ;
        self.range_tc_types(typ)
    }

    pub fn get_return_tc_types(&mut self, e: &Expr) -> Vec<TCTypeKey> {
        let typ = self.ti.types.get(&e.id()).unwrap().typ;
        self.return_tc_types(typ)
    }

    pub fn get_selection_value_types(&mut self, id: NodeId) -> (ValueType, ValueType) {
        let sel = &self.ti.selections[&id];
        let t0 = self.value_type_from_tc(sel.recv().unwrap());
        let t1 = self.value_type_from_tc(self.tc_objs.lobjs[sel.obj()].typ().unwrap());
        (t0, t1)
    }

    pub fn meta_from_tc(&mut self, typ: TCTypeKey, vm_objs: &mut VMObjects) -> GosMetadata {
        if !self.types_cache.contains_key(&typ) {
            let val = self.meta_from_tc_impl(typ, vm_objs);
            self.types_cache.insert(typ, val);
        }
        self.types_cache.get(&typ).unwrap().clone()
    }

    pub fn get_sig_params_tc_types(
        &mut self,
        func: TCTypeKey,
    ) -> (Vec<TCTypeKey>, Option<(TCTypeKey, TCTypeKey)>) {
        let typ = &self.tc_objs.types[func].underlying_val(self.tc_objs);
        let sig = typ.try_as_signature().unwrap();
        let params: Vec<TCTypeKey> = self.tc_objs.types[sig.params()]
            .try_as_tuple()
            .unwrap()
            .vars()
            .iter()
            .map(|&x| self.tc_objs.lobjs[x].typ().unwrap())
            .collect();
        let variadic = if sig.variadic() {
            let slice_key = *params.last().unwrap();
            let slice = &self.tc_objs.types[slice_key].try_as_slice().unwrap();
            Some((slice_key, slice.elem()))
        } else {
            None
        };
        (params, variadic)
    }

    pub fn get_sig_returns_tc_types(&mut self, func: TCTypeKey) -> Vec<TCTypeKey> {
        let typ = &self.tc_objs.types[func].underlying_val(self.tc_objs);
        let sig = typ.try_as_signature().unwrap();
        self.return_tc_types(sig.results())
    }

    // returns vm_type(metadata) for the tc_type
    fn basic_type_from_tc(&self, tkey: TCTypeKey, vm_objs: &mut VMObjects) -> GosMetadata {
        let typ = self.tc_objs.types[tkey].try_as_basic().unwrap().typ();
        match typ {
            BasicType::Bool | BasicType::UntypedBool => vm_objs.metadata.mbool.clone(),
            BasicType::Int | BasicType::UntypedInt => vm_objs.metadata.mint.clone(),
            BasicType::Int8 => vm_objs.metadata.mint8.clone(),
            BasicType::Int16 => vm_objs.metadata.mint16.clone(),
            BasicType::Int32 | BasicType::Rune | BasicType::UntypedRune => {
                vm_objs.metadata.mint32.clone()
            }
            BasicType::Int64 => vm_objs.metadata.mint64.clone(),
            BasicType::Uint | BasicType::Uintptr | BasicType::UnsafePointer => {
                vm_objs.metadata.muint.clone()
            }
            BasicType::Uint8 | BasicType::Byte => vm_objs.metadata.muint8.clone(),
            BasicType::Uint16 => vm_objs.metadata.muint16.clone(),
            BasicType::Uint32 => vm_objs.metadata.muint32.clone(),
            BasicType::Uint64 => vm_objs.metadata.muint64.clone(),
            BasicType::Float32 => vm_objs.metadata.mfloat32.clone(),
            BasicType::Float64 | BasicType::UntypedFloat => vm_objs.metadata.mfloat64.clone(),
            BasicType::Complex64 => vm_objs.metadata.mcomplex64.clone(),
            BasicType::Complex128 => vm_objs.metadata.mcomplex128.clone(),
            BasicType::Str | BasicType::UntypedString => vm_objs.metadata.mstr.clone(),
            BasicType::UntypedNil => GosMetadata::Untyped,
            _ => {
                dbg!(typ);
                unreachable!()
            }
        }
    }

    // get GosValue from type checker's Obj
    fn const_value(&self, tkey: TCTypeKey, val: &ConstValue) -> GosValue {
        let typ = self.tc_objs.types[tkey]
            .underlying_val(self.tc_objs)
            .try_as_basic()
            .unwrap()
            .typ();
        match typ {
            BasicType::Bool | BasicType::UntypedBool => GosValue::Bool(val.bool_as_bool()),
            BasicType::Int | BasicType::UntypedInt => {
                let (i, _) = val.to_int().int_as_i64();
                GosValue::Int(i as isize)
            }
            BasicType::Int8 => {
                let (i, _) = val.to_int().int_as_i64();
                GosValue::Int8(i as i8)
            }
            BasicType::Int16 => {
                let (i, _) = val.to_int().int_as_i64();
                GosValue::Int16(i as i16)
            }
            BasicType::Int32 | BasicType::Rune | BasicType::UntypedRune => {
                let (i, _) = val.to_int().int_as_i64();
                GosValue::Int32(i as i32)
            }
            BasicType::Int64 => {
                let (i, _) = val.to_int().int_as_i64();
                GosValue::Int64(i)
            }
            BasicType::Uint | BasicType::Uintptr | BasicType::UnsafePointer => {
                let (i, _) = val.to_int().int_as_u64();
                GosValue::Uint(i as usize)
            }
            BasicType::Uint8 => {
                let (i, _) = val.to_int().int_as_u64();
                GosValue::Uint8(i as u8)
            }
            BasicType::Uint16 => {
                let (i, _) = val.to_int().int_as_u64();
                GosValue::Uint16(i as u16)
            }
            BasicType::Uint32 => {
                let (i, _) = val.to_int().int_as_u64();
                GosValue::Uint32(i as u32)
            }
            BasicType::Uint64 => {
                let (i, _) = val.to_int().int_as_u64();
                GosValue::Uint64(i)
            }
            BasicType::Float32 => {
                let (f, _) = val.num_as_f32();
                GosValue::Float32(f.into())
            }
            BasicType::Float64 | BasicType::UntypedFloat => {
                let (f, _) = val.num_as_f64();
                GosValue::Float64(f.into())
            }
            BasicType::Complex64 => {
                let (cr, ci, _) = val.complex_as_complex64();
                GosValue::Complex64(cr, ci)
            }
            BasicType::Complex128 => {
                let (cr, ci, _) = val.complex_as_complex128();
                GosValue::Complex128(Box::new((cr, ci)))
            }
            BasicType::Str | BasicType::UntypedString => GosValue::new_str(val.str_as_string()),
            _ => unreachable!(),
        }
    }

    // get vm_type from tc_type
    fn meta_from_tc_impl(&mut self, typ: TCTypeKey, vm_objs: &mut VMObjects) -> GosMetadata {
        match &self.tc_objs.types[typ] {
            Type::Basic(_) => self.basic_type_from_tc(typ, vm_objs),
            Type::Array(detail) => {
                let elem = self.meta_from_tc_impl(detail.elem(), vm_objs);
                GosMetadata::new_array(elem, detail.len().unwrap() as usize, &mut vm_objs.metas)
            }
            Type::Slice(detail) => {
                let el_type = self.meta_from_tc(detail.elem(), vm_objs);
                GosMetadata::new_slice(el_type, &mut vm_objs.metas)
            }
            Type::Map(detail) => {
                let ktype = self.meta_from_tc(detail.key(), vm_objs);
                let vtype = self.meta_from_tc(detail.elem(), vm_objs);
                GosMetadata::new_map(ktype, vtype, &mut vm_objs.metas)
            }
            Type::Struct(detail) => {
                let fields = self.get_fields(detail.fields(), vm_objs);
                GosMetadata::new_struct(fields, vm_objs)
            }
            Type::Interface(detail) => {
                let methods = detail.all_methods();
                let fields = self.get_fields(methods.as_ref().unwrap(), vm_objs);
                GosMetadata::new_interface(fields, &mut vm_objs.metas)
            }
            Type::Signature(detail) => {
                let mut convert = |tuple_key| -> Vec<GosMetadata> {
                    self.tc_objs.types[tuple_key]
                        .try_as_tuple()
                        .unwrap()
                        .vars()
                        .iter()
                        .map(|&x| self.meta_from_tc(self.tc_objs.lobjs[x].typ().unwrap(), vm_objs))
                        .collect()
                };
                let params = convert(detail.params());
                let results = convert(detail.results());
                let mut recv = None;
                if let Some(r) = detail.recv() {
                    let recv_tc_type = self.tc_objs.lobjs[*r].typ().unwrap();
                    // to avoid infinite recursion
                    if !self.tc_objs.types[recv_tc_type].is_interface(self.tc_objs) {
                        recv = Some(self.meta_from_tc(recv_tc_type, vm_objs));
                    }
                }
                let variadic = if detail.variadic() {
                    let slice = params.last().unwrap();
                    match &vm_objs.metas[slice.as_non_ptr()] {
                        MetadataType::Slice(elem) => Some((slice.clone(), elem.clone())),
                        _ => unreachable!(),
                    }
                } else {
                    None
                };
                GosMetadata::new_sig(recv, params, results, variadic, &mut vm_objs.metas)
            }
            Type::Pointer(detail) => {
                let inner = self.meta_from_tc(detail.base(), vm_objs);
                inner.ptr_to()
            }
            Type::Named(detail) => {
                let underlying = self.meta_from_tc(detail.underlying(), vm_objs);
                let md = GosMetadata::new_named(underlying, &mut vm_objs.metas);
                for key in detail.methods().iter() {
                    let mobj = &self.tc_objs.lobjs[*key];
                    md.add_method(
                        mobj.name().clone(),
                        mobj.entity_type().func_has_ptr_recv(),
                        &mut vm_objs.metas,
                    )
                }
                md
            }
            _ => {
                dbg!(&self.tc_objs.types[typ]);
                unimplemented!()
            }
        }
    }

    pub fn underlying_value_type_from_tc(&self, typ: TCTypeKey) -> ValueType {
        match &self.tc_objs.types[typ] {
            Type::Named(n) => self.value_type_from_tc(n.underlying()),
            _ => self.value_type_from_tc(typ),
        }
    }

    pub fn value_type_from_tc(&self, typ: TCTypeKey) -> ValueType {
        match &self.tc_objs.types[typ] {
            Type::Basic(detail) => match detail.typ() {
                BasicType::Bool | BasicType::UntypedBool => ValueType::Bool,
                BasicType::Int | BasicType::UntypedInt => ValueType::Int,
                BasicType::Int8 => ValueType::Int8,
                BasicType::Int16 => ValueType::Int16,
                BasicType::Int32 | BasicType::Rune | BasicType::UntypedRune => ValueType::Int32,
                BasicType::Int64 => ValueType::Int64,
                BasicType::Uint | BasicType::Uintptr | BasicType::UnsafePointer => ValueType::Uint,
                BasicType::Uint8 | BasicType::Byte => ValueType::Uint8,
                BasicType::Uint16 => ValueType::Uint16,
                BasicType::Uint32 => ValueType::Uint32,
                BasicType::Uint64 => ValueType::Uint64,
                BasicType::Float32 => ValueType::Float32,
                BasicType::Float64 | BasicType::UntypedFloat => ValueType::Float64,
                BasicType::Complex64 => ValueType::Complex64,
                BasicType::Complex128 => ValueType::Complex128,
                BasicType::Str | BasicType::UntypedString => ValueType::Str,
                BasicType::UntypedNil => ValueType::Nil,
                _ => {
                    dbg!(detail.typ());
                    unreachable!()
                }
            },
            Type::Array(_) => ValueType::Array,
            Type::Slice(_) => ValueType::Slice,
            Type::Map(_) => ValueType::Map,
            Type::Struct(_) => ValueType::Struct,
            Type::Interface(_) => ValueType::Interface,
            Type::Signature(_) => ValueType::Closure,
            Type::Pointer(_) => ValueType::Pointer,
            Type::Named(_) => ValueType::Named,
            _ => {
                dbg!(&self.tc_objs.types[typ]);
                unimplemented!()
            }
        }
    }

    fn range_tc_types(&self, typ: TCTypeKey) -> [TCTypeKey; 3] {
        let t_int = self.tc_objs.universe().types()[&BasicType::Int];
        match &self.tc_objs.types[typ] {
            Type::Basic(detail) => match detail.typ() {
                BasicType::Str | BasicType::UntypedString => [typ, t_int, t_int],
                _ => unreachable!(),
            },
            Type::Slice(detail) => [typ, t_int, detail.elem()],
            Type::Map(detail) => [typ, detail.key(), detail.elem()],
            _ => {
                dbg!(&self.tc_objs.types[typ]);
                unreachable!()
            }
        }
    }

    fn return_tc_types(&self, typ: TCTypeKey) -> Vec<TCTypeKey> {
        match &self.tc_objs.types[typ] {
            Type::Tuple(detail) => detail
                .vars()
                .iter()
                .map(|x| self.tc_objs.lobjs[*x].typ().unwrap())
                .collect(),
            _ => unreachable!(),
        }
    }

    fn get_fields(&mut self, fields: &Vec<ObjKey>, vm_objs: &mut VMObjects) -> Fields {
        let mut vec = Vec::new();
        let mut map = HashMap::<String, OpIndex>::new();
        for (i, f) in fields.iter().enumerate() {
            let field = &self.tc_objs.lobjs[*f];
            let f_type = self.meta_from_tc(field.typ().unwrap(), vm_objs);
            vec.push(f_type);
            map.insert(field.name().clone(), i as OpIndex);
        }
        Fields::new(vec, map)
    }
}
