use crate::CRATE;
use darling::{
    util::{Flag, SpannedValue},
    FromDeriveInput, FromField, FromMeta,
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput};
use syn::spanned::Spanned;
use crate::struct_layout::ensure_repr_c_derive_input;

const VK_DESCRIPTOR_UPDATE_DATA_LEN: usize = 24;
const VK_DESCRIPTOR_UPDATE_DATA_LEN: usize = 24;
const VK_DESCRIPTOR_UPDATE_DATA_LEN: usize = 24;

#[derive(FromDeriveInput, Debug)]
#[darling(forward_attrs(allow, doc, cfg, repr))]
struct ArgumentsStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
}

#[derive(Default, FromMeta)]
#[darling(default)]
struct StagesMeta {
    all_graphics: Flag,
    compute: Flag,
    vertex: Flag,
    fragment: Flag,
    geometry: Flag,
    tessellation_control: Flag,
    tessellation_evaluation: Flag,
}

#[derive(Default, FromMeta)]
struct RuntimeArrayMeta {
    max_count: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ArgumentKind {
    Uniform,
    Sampler,
    SampledImage,
    StorageImage,
    UniformBuffer,
    UniformBufferDynamic,
    StorageBuffer,
    StorageBufferDynamic,
}

impl ArgumentKind {
    fn descriptor_type(&self) -> Option<TokenStream> {
        match self {
            ArgumentKind::Sampler => Some(quote! { #CRATE::vk::DescriptorType::SAMPLER }),
            ArgumentKind::SampledImage => {
                Some(quote! { #CRATE::vk::DescriptorType::SAMPLED_IMAGE })
            }
            ArgumentKind::StorageImage => {
                Some(quote! { #CRATE::vk::DescriptorType::STORAGE_IMAGE })
            }
            ArgumentKind::UniformBuffer => {
                Some(quote! { #CRATE::vk::DescriptorType::UNIFORM_BUFFER })
            }
            ArgumentKind::UniformBufferDynamic => {
                Some(quote! { #CRATE::vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC })
            }
            ArgumentKind::StorageBuffer => {
                Some(quote! { #CRATE::vk::DescriptorType::STORAGE_BUFFER })
            }
            ArgumentKind::StorageBufferDynamic => {
                Some(quote! { #CRATE::vk::DescriptorType::STORAGE_BUFFER_DYNAMIC })
            }
            ArgumentKind::Uniform => None,
        }
    }
}

enum DescriptorWriteType {
    Image,
    Buffer,
}

#[derive(FromField)]
#[darling(attributes(argument))]
struct ArgumentAttr {
    #[darling(default)]
    binding: u32,
    #[darling(default)]
    runtime_array: Option<SpannedValue<RuntimeArrayMeta>>,
    #[darling(default)]
    uniform: Flag,
}

fn generate_shader_access_mask(stages: &SpannedValue<StagesMeta>) -> TokenStream {
    let mut flags = Vec::new();
    if stages.all_graphics.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::ALL_GRAPHICS});
    }
    if stages.vertex.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::VERTEX});
    }
    if stages.fragment.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::FRAGMENT});
    }
    if stages.geometry.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::GEOMETRY});
    }
    if stages.tessellation_control.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::TESSELLATION_CONTROL});
    }
    if stages.tessellation_evaluation.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::TESSELLATION_EVALUATION});
    }
    if stages.compute.is_some() {
        flags.push(quote! {#CRATE::vk::ShaderStageFlags::COMPUTE});
    }
    if flags.is_empty() {
        syn::Error::new(stages.span(), "No shader stage specified. Expected one or more of `all_graphics`, `compute`, `vertex`, `fragment`, `geometry`, `tessellation_control`, `tessellation_evaluation`.").to_compile_error()
    } else {
        quote! {
            #CRATE::vk::ShaderStageFlags::from_raw(0 #(| #flags.as_raw())*)
        }
    }
}

struct Binding<'a> {
    field: &'a syn::Field,
    field_index: usize,
    binding: u32,
}


struct Arguments<'a> {
    bindings: Vec<Binding<'a>>,
    default_uniform_buffer_fields: Vec<&'a syn::Field>,
}

impl<'a> Arguments<'a> {
    fn parse(derive_input: &'a syn::DeriveInput, fields: &'a FieldList) -> Arguments<'a> {
        let s: ArgumentsStruct =
            <ArgumentsStruct as FromDeriveInput>::from_derive_input(derive_input).unwrap();

        //let struct_name = &s.ident;
        //let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();

        let mut bindings = Vec::new();


        Arguments {
            bindings,
            default_uniform_buffer_fields,
        }
    }

    fn generate_default_uniform_buffer_struct(&self) -> TokenStream {
        let fields = &self.default_uniform_buffer_fields;
        quote! {
            #[repr(C)]
            struct DefaultUniformBuffer {
                #(#fields,)*
            }
        }
    }

    fn generate_vk_descriptor_set_layout_bindings(&self) -> TokenStream {
        let bindings: Vec<_> = self
            .bindings
            .iter()
            .map(|b| b.generate_vk_descriptor_set_layout_binding())
            .collect();

        quote! {
            [
                #(#bindings,)*
            ]
        }
    }

    fn generate_shader_arguments(&self) -> TokenStream {
        let descriptor_update_template_entries = self.generate_vk_descriptor_update_template_entries();

        // generate update data struct
        let mut descriptor_update_struct_fields = Vec::new();
        if !self.default_uniform_buffer_fields.is_empty() {
            descriptor_update_struct_fields.push(quote! { #CRATE::vk::DescriptorBufferInfo });
        }
        for b in self.bindings.iter() {
            let binding = b.binding;
            let ty = &b.field.ty;
            descriptor_update_struct_fields.push(quote! { <#ty as #CRATE::DescriptorBinding>::DescriptorUpdateDataType });
        }




    }



    fn generate_vk_descriptor_update_template_entries(&self) -> TokenStream {

        // generate update template entries
         {
            let mut offset = quote! { 0 };
            let mut num_entries = quote! { 0 };

            if !self.default_uniform_buffer_fields.is_empty() {
                // generate a template entry for the default uniform buffer
                descriptor_update_template_entries.push(quote! {
                    #CRATE::vk::DescriptorUpdateTemplateEntry {
                        dst_binding       : 0,
                        dst_array_element : 0,
                        descriptor_count  : 1,
                        descriptor_type   : #CRATE::vk::DescriptorType::UNIFORM_BUFFER,
                        offset            : #offset,
                        stride            : 24      // sizeof<VkDescriptorBufferInfo>
                    }
                });
                offset = quote! { #offset + 24 };
            }

            for b in self.bindings.iter() {
                let binding = b.binding;
                let ty = &b.field.ty;
                descriptor_update_template_entries.push(quote! {
                    #CRATE::vk::DescriptorUpdateTemplateEntry {
                        dst_binding       : #binding,
                        dst_array_element : 0,
                        descriptor_count  : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT,
                        descriptor_type   : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_TYPE,
                        offset            : #offset,
                        stride            : ::std::mem::size_of::<<#ty as #CRATE::DescriptorBinding>::DescriptorUpdateDataType>()
                    }
                });
                offset = quote! { #offset + ::std::mem::size_of::<<#ty as #CRATE::DescriptorBinding>::DescriptorUpdateDataType>() * <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT };
            }
        }




        // generate code for filling the descriptor update data array
        {
            let mut descriptor_update_data_stmts = Vec::new();
            if !self.default_uniform_buffer_fields.is_empty() {
                descriptor_update_data_stmts.push(quote! {
                    update_data.0 = todo!();
                });
            }


        }



        for b in self.bindings.iter() {
            let size = match b.descriptor_write_type() {
                DescriptorWriteType::Image => VK_DESCRIPTOR_IMAGE_INFO_LEN,
                DescriptorWriteType::Buffer => VK_DESCRIPTOR_BUFFER_INFO_LEN,
            };

            let name = b.field.ident.as_ref().unwrap();
            let binding = b.binding;
            let descriptor_type = &b.descriptor_type;
            let field_ty = &b.field.ty;
            let offset_0 = syn::Ident::new(&format!("OFFSET_{}", i), Span::call_site());
            let descriptor_write_type = quote! { <#field_ty as #CRATE::DescriptorSource>::DescriptorWriteType };

            update_template_entries.push(quote! {
            });

            descriptor_update_fields.push(quote! {
                #name: #descriptor_write_type
            });

            descriptor_update_stmts.push(quote! {
                 update_data.#name = #CRATE::DescriptorSource::to_descriptors(arguments.#name);
            });

            if i < self.bindings.len() - 1 {
                let ty = &descriptor_write_type;
                let offset_1 = syn::Ident::new(&format!("OFFSET_{}", i + 1), Span::call_site());
                let offset_const = quote! {
                    pub const #offset_1: usize = (#offset_0 + ::std::mem::size_of::<#ty>()) + (::std::mem::align_of::<#ty>() - (#offset_0 + ::std::mem::size_of::<#ty>()) % ::std::mem::align_of::<#ty>()) % ::std::mem::align_of::<#ty>();
                };
                descriptor_write_offset_consts.push(offset_const);
            }
        }

        let descriptor_update_data_struct = quote! {
            #[repr(C)]
            #[derive(Copy,Clone,Debug)]
            struct DescriptorUpdateData {
                #(#descriptor_update_data_fields,)*
            }
        };


        let update_template_entries = quote! {
            {
                #(#descriptor_write_offset_consts)*
                [
                    #(#update_template_entries,)*
                ]
            }
        };

        let descriptor_update_struct = quote! {
            #[repr(C)]
            #[derive(Copy,Clone)]
            struct DescriptorUpdateData {
                #(#descriptor_update_fields,)*
            }
        };

        todo!()
    }
}

pub fn derive(input: proc_macro::TokenStream) -> Result<TokenStream, syn::Error> {

    let derive_input: syn::DeriveInput = syn::parse(input)?;

    // check for struct
    let fields = match derive_input.data {
        Data::Struct(ref struct_data) => {
            &struct_data.fields
        }
        _ => {
            return Err(syn::Error::new(derive_input.span(), "`ShaderArguments` can only be derived on structs"));
        }
    };
    // check for `#[repr(C)]`
    ensure_repr_c_derive_input(&derive_input)?;

    // parse struct-level `#[arguments(...)]` attribs.
    let s: ArgumentsStruct =
        <ArgumentsStruct as FromDeriveInput>::from_derive_input(&derive_input).unwrap();

    //let struct_name = &s.ident;
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();

    let mut bindings = Vec::new();
    let mut update_template_entries = Vec::new();
    let mut runtime_array_writes = Vec::new();
    let mut default_uniform_buffer_fields = Vec::new();

    // parse field bindings and inline uniform fields
    for (field_index, field) in fields.iter().enumerate() {
        match <ArgumentAttr as FromField>::from_field(field) {
            Ok(attr) => {
                let attr: &ArgumentAttr = &attr;
                // TODO verify attrs
                if attr.uniform.is_some() {
                    // this is an inline uniform
                    default_uniform_buffer_fields.push(field);
                } else {
                    // this is an actual binding
                    bindings.push(Binding { field, field_index, binding: attr.binding })
                }
            }
        }
    }

    // binding #0 is reserved for the default uniform buffer is there's one: ensure it's not used
    if !default_uniform_buffer_fields.is_empty() {
        if let Some(binding_zero) = bindings.iter().find(|b| b.binding == 0) {
            // TODO better warning
            binding_zero
                .span()
                .unwrap()
                .error("Binding number 0 is reserved for inline uniforms")
                .emit();
        }
    }

    // --- Generate descriptor set layout bindings ---
    let descriptor_set_layout_bindings: Vec<_> = bindings
        .iter()
        .map(|b| {
            let ty = &b.field.ty;
            let binding = b.binding;
            quote! {
                #CRATE::vk::DescriptorSetLayoutBinding {
                    binding           : #binding,
                    stage_flags       : <#ty as #CRATE::DescriptorBinding>::STAGE_FLAGS,
                    descriptor_type   : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_TYPE,
                    descriptor_count  : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT,
                    immutable_samplers: ::std::ptr::null()
                }
            }
        }).collect();

    // --- Generate update template entries ---
    let descriptor_update_template_entries : Vec<_> = {
        let mut entries = Vec::new();
        let mut offset = quote! { 0 };

        if !default_uniform_buffer_fields.is_empty() {
            // generate a template entry for the default uniform buffer
            entries.push(quote! {
                    #CRATE::vk::DescriptorUpdateTemplateEntry {
                        dst_binding       : 0,
                        dst_array_element : 0,
                        descriptor_count  : 1,
                        descriptor_type   : #CRATE::vk::DescriptorType::UNIFORM_BUFFER,
                        offset            : #offset,
                        stride            : 24      // sizeof<VkDescriptorBufferInfo>
                    }
                });
            offset = quote! { #offset + 24 };
        }

        for b in bindings.iter() {
            let binding = b.binding;
            let ty = &b.field.ty;
            entries.push(quote! {
                    #CRATE::vk::DescriptorUpdateTemplateEntry {
                        dst_binding       : #binding,
                        dst_array_element : 0,
                        descriptor_count  : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT,
                        descriptor_type   : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_TYPE,
                        offset            : #offset,
                        stride            : ::std::mem::size_of::<<#ty as #CRATE::DescriptorBinding>::DescriptorUpdateDataType>()
                    }
                });
            offset = quote! { #offset + ::std::mem::size_of::<<#ty as #CRATE::DescriptorBinding>::DescriptorUpdateDataType>() * <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT };
        }

        entries
    };

    // --- generate update data struct
    let descriptor_update_struct_fields = {
        let mut fields = Vec::new();
        if !default_uniform_buffer_fields.is_empty() {
            fields.push(quote! { #CRATE::vk::DescriptorBufferInfo });
        }
        for b in bindings.iter() {
            let binding = b.binding;
            let ty = &b.field.ty;
            fields.push(quote! { <#ty as #CRATE::DescriptorBinding>::DescriptorUpdateDataType });
        }
        fields
    };


    // --- Generate update data initializers
    let descriptor_update_data_stmts = {
        let mut stmts = Vec::new();
        let mut i = 0;
        if !default_uniform_buffer_fields.is_empty() {
            stmts.push(quote! { todo!() });
            i += 1;
        }

        for b in bindings.iter() {
            let binding = b.binding;
            let name = &b.field.ident;
            let ty = &b.field.ty;
            let field = syn::Index::from(i);
            stmts.push(quote! {
                self.#name.write_descriptors(ctx, if <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT == 1 { &mut update_data.#field } else { ::std::slice::from_mut(&mut update_data.#field) });
            });
        }
    };

    let q = quote! {
        impl #impl_generics #CRATE::ShaderArguments for #struct_name #ty_generics #where_clause {

            fn get_or_create_descriptor_set_layout(&self, cache: &mut #CRATE::DescriptorSetLayoutCache) -> #CRATE::DescriptorSetLayoutId {
                const BINDINGS: &'static [#CRATE::vk::DescriptorSetLayoutBinding] = &[ #(#descriptor_set_layout_bindings,)* ];
                const UPDATE_TEMPLATE_ENTRIES: &'static [#CRATE::vk::DescriptorUpdateTemplateEntry] = &[#(#descriptor_update_template_entries,)*];
                cache.get_or_create_descriptor_set_layout(Some(::std::any::TypeId::of::<Self>()), )
            }


            unsafe fn update_descriptor_set(&self,
                             ctx: &mut #CRATE::PassSubmitCtx,
                             device: &#CRATE::ash::Device,
                             set: #CRATE::vk::DescriptorSet,
                             update_template: Option<#CRATE::vk::DescriptorUpdateTemplate>)
            {
                if !Self::UPDATE_TEMPLATE_ENTRIES.is_empty() {
                    device.update_descriptor_set_with_template(set, update_template, self as *const _ as *const ::std::ffi::c_void);
                }

                /*// write runtime-sized arrays
                let descriptor_writes = &[#(#runtime_array_writes,)*];
                if !descriptor_writes.is_empty() {
                    device.update_descriptor_sets(descriptor_writes, &[]);
                }*/
            }
        }
    };
    q
}
