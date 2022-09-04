//! SPIR-V Backend
use crate::{
    ast,
    ast::{Expr, Id, TypeDesc},
};
use rspirv::{
    spirv,
    spirv::{FunctionControl, Word},
};

struct SpirvEmitter<'a> {
    module: &'a ast::Module,
    builder: rspirv::dr::Builder,

    function_map: Vec<Word>,
    type_map: Vec<Word>,

    base_id: u32,
}

impl<'a> SpirvEmitter<'a> {
    fn type_result_id(&self, ty: Id<TypeDesc>) -> Word {
        self.type_map[ty.index()]
    }

    fn prim_type_result_id(&self, prim_ty: ast::PrimitiveType) -> Word {
        match prim_ty {
            ast::PrimitiveType::Int => self.type_result_id(self.module.i32_type),
            ast::PrimitiveType::UnsignedInt => self.type_result_id(self.module.u32_type),
            ast::PrimitiveType::Float => self.type_result_id(self.module.f32_type),
            ast::PrimitiveType::Double => self.type_result_id(self.module.f64_type),
            ast::PrimitiveType::Bool => self.type_result_id(self.module.bool_type),
        }
    }

    fn vector_type_result_id(&self, prim_ty: ast::PrimitiveType, len: u32) -> Word {
        match len {
            2 => match prim_ty {
                ast::PrimitiveType::Int => self.type_result_id(self.module.i32x2_type),
                ast::PrimitiveType::UnsignedInt => self.type_result_id(self.module.u32x2_type),
                ast::PrimitiveType::Float => self.type_result_id(self.module.f32x2_type),
                ast::PrimitiveType::Double => todo!(),
                ast::PrimitiveType::Bool => self.type_result_id(self.module.bool2_type),
            },
            3 => match prim_ty {
                ast::PrimitiveType::Int => self.type_result_id(self.module.i32x3_type),
                ast::PrimitiveType::UnsignedInt => self.type_result_id(self.module.u32x3_type),
                ast::PrimitiveType::Float => self.type_result_id(self.module.f32x3_type),
                ast::PrimitiveType::Double => todo!(),
                ast::PrimitiveType::Bool => self.type_result_id(self.module.bool3_type),
            },
            4 => match prim_ty {
                ast::PrimitiveType::Int => self.type_result_id(self.module.i32x4_type),
                ast::PrimitiveType::UnsignedInt => self.type_result_id(self.module.u32x4_type),
                ast::PrimitiveType::Float => self.type_result_id(self.module.f32x4_type),
                ast::PrimitiveType::Double => todo!(),
                ast::PrimitiveType::Bool => self.type_result_id(self.module.bool4_type),
            },
            _ => panic!("invalid vector size"),
        }
    }

    fn emit_type(&mut self, id: Id<TypeDesc>, ty: &ast::TypeDesc) -> Word {
        match *ty {
            ast::TypeDesc::Void => self.builder.type_void(),
            ast::TypeDesc::Primitive(prim_ty) => match prim_ty {
                ast::PrimitiveType::Int => self.builder.type_int(32, 1),
                ast::PrimitiveType::UnsignedInt => self.builder.type_int(32, 0),
                ast::PrimitiveType::Float => self.builder.type_float(32),
                ast::PrimitiveType::Double => self.builder.type_float(64),
                ast::PrimitiveType::Bool => self.builder.type_bool(),
            },
            ast::TypeDesc::Vector { elem_ty, len } => {
                let elem_ty_id = self.prim_type_result_id(elem_ty);
                self.builder.type_vector(elem_ty_id, len as u32)
            }
            ast::TypeDesc::Matrix { elem_ty, rows, columns } => {
                let column_type = self.vector_type_result_id(elem_ty, rows as u32);
                self.builder.type_matrix(column_type, columns as u32)
            }
            ast::TypeDesc::Array { .. } => {
                todo!()
            }
            ast::TypeDesc::RuntimeArray(_) => {
                todo!()
            }
            ast::TypeDesc::Struct(_) => {
                todo!()
            }
            ast::TypeDesc::SampledImage(_) => {
                todo!()
            }
            ast::TypeDesc::Image(_) => {
                todo!()
            }
            ast::TypeDesc::Pointer(_) => {
                todo!()
            }
            ast::TypeDesc::Sampler => {
                todo!()
            }
            ast::TypeDesc::ShadowSampler => {
                todo!()
            }
            ast::TypeDesc::String => {
                todo!()
            }
            ast::TypeDesc::Unknown => {
                todo!()
            }
            ast::TypeDesc::Function {
                return_type,
                ref arguments,
            } => {
                let return_type = self.type_result_id(return_type);
                let arguments = arguments
                    .iter()
                    .map(|&arg| self.type_result_id(arg))
                    .collect::<Vec<_>>();
                self.builder.type_function(return_type, arguments)
            }
            TypeDesc::Error => {
                // Modules always contain the error type, but as long as it's never referenced it's OK
                // Just use a dummy type
                self.builder.type_void()
            }
        }
    }

    fn emit_function(&mut self, function: &ast::Function) {
        let function_type = self.type_result_id(function.function_type);
        let return_type = match self.module.types[function.function_type] {
            TypeDesc::Function { return_type, .. } => return_type,
            _ => panic!("malformed module"),
        };
        let return_type = self.type_result_id(return_type);

        self.builder
            .begin_function(return_type, Some(self.base_id), FunctionControl::NONE, function_type)
            .unwrap();

        let mut map = vec![0u32; function.exprs.len()];

        for (i, expr) in function.exprs.iter().enumerate() {
            let result_type = function.types[i].map(|id| self.type_result_id(id));
            let remap = |id: Id<Expr>| -> u32 { map[id.index()] };

            let result_id = match *expr {
                Expr::Argument { index } => {
                    todo!()
                    //let param_type = self.type_result_id(ty);
                    //self.builder.function_parameter(param_type).unwrap()
                }
                Expr::AccessField { place, index } => {
                    let place = remap(place);
                    self.builder
                        .access_chain(result_type.unwrap(), None, place, [index])
                        .unwrap()
                }
                Expr::AccessIndex { place, index } => {
                    let place = remap(place);
                    let index = remap(index);
                    self.builder
                        .access_chain(result_type.unwrap(), None, place, [index])
                        .unwrap()
                }
                Expr::Load { pointer } => {
                    let place = remap(pointer);
                    self.builder.load(result_type.unwrap(), None, place, None, []).unwrap()
                }
                Expr::LocalVariable { ref name, ty, init } => {
                    let ty = self.type_result_id(ty);
                    let init = init.map(remap);
                    self.builder.variable(ty, None, spirv::StorageClass::Function, init)
                }
                Expr::Store { place, expr } => {
                    let place = remap(place);
                    let expr = remap(expr);
                    self.builder.store(place, expr, None, []).unwrap();
                    continue;
                }
                Expr::Apply { .. } => {
                    todo!()
                }
                Expr::Not { .. } => {
                    todo!()
                }
                Expr::FAdd { left, right } => {
                    let left = remap(left);
                    let right = remap(right);
                    self.builder.f_add(result_type.unwrap(), None, left, right).unwrap()
                }
                Expr::FSub { .. } => {
                    todo!()
                }
                Expr::FMul { .. } => {
                    todo!()
                }
                Expr::FDiv { .. } => {
                    todo!()
                }
                Expr::Mod { .. } => {
                    todo!()
                }
                Expr::Shl { .. } => {
                    todo!()
                }
                Expr::Shr { .. } => {
                    todo!()
                }
                Expr::Or { .. } => {
                    todo!()
                }
                Expr::And { .. } => {
                    todo!()
                }
                Expr::BitOr { .. } => {
                    todo!()
                }
                Expr::BitAnd { .. } => {
                    todo!()
                }
                Expr::BitXor { .. } => {
                    todo!()
                }
                Expr::Eq { .. } => {
                    todo!()
                }
                Expr::Ne { .. } => {
                    todo!()
                }
                Expr::Lt { .. } => {
                    todo!()
                }
                Expr::Le { .. } => {
                    todo!()
                }
                Expr::Gt { .. } => {
                    todo!()
                }
                Expr::Ge { .. } => {
                    todo!()
                }
                Expr::ArrayIndex { .. } => {
                    todo!()
                }
                Expr::CompositeConstruct { .. } => {
                    todo!()
                }
                Expr::I32Const(_) => {
                    todo!()
                }
                Expr::U32Const(_) => {
                    todo!()
                }
                Expr::BoolConst(_) => {
                    todo!()
                }
                Expr::F32Const(_) => {
                    todo!()
                }
                Expr::F64Const(_) => {
                    todo!()
                }
                Expr::Error => {
                    todo!()
                }
                Expr::Loop { .. } => {
                    todo!()
                }
                Expr::Selection { .. } => {
                    todo!()
                }
                Expr::Merge(_) => {
                    todo!()
                }
                Expr::Continue(_) => {
                    todo!()
                }
                Expr::Label => {
                    todo!()
                }
                Expr::Branch => {
                    todo!()
                }
                Expr::Noop => {
                    todo!()
                }
                Expr::Return(_) => {
                    todo!()
                }
                Expr::Discard => {
                    todo!()
                }
                Expr::IAdd { .. } => {
                    todo!()
                }
                Expr::ISub { .. } => {
                    todo!()
                }
                Expr::IMul { .. } => {
                    todo!()
                }
                Expr::IDiv { .. } => {
                    todo!()
                }
                Expr::EndFunction => break,
                Expr::FNeg { .. } => {
                    todo!()
                }
                Expr::SNeg { .. } => {
                    todo!()
                }
                Expr::Global { .. } => {
                    todo!()
                }
            };
            map[i] = result_id;
        }
    }
}

fn emit_spirv(module: &ast::Module) -> rspirv::dr::Module {
    let mut b = rspirv::dr::Builder::new();
    b.set_version(1, 0);
    b.module()
}
