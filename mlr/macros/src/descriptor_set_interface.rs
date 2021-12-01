use crate::{struct_layout::ensure_repr_c_derive_input, CRATE};
use darling::{
    util::{Flag, SpannedValue},
    FromDeriveInput, FromField, FromMeta,
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput};

const VK_DESCRIPTOR_UPDATE_DATA_LEN: usize = 24;
const VK_DESCRIPTOR_IMAGE_INFO_LEN: usize = 24;
const VK_DESCRIPTOR_BUFFER_INFO_LEN: usize = 24;

#[derive(FromDeriveInput, Debug)]
#[darling(forward_attrs(allow, doc, cfg, repr))]
struct ArgumentsStruct {
    ident: syn::Ident,
    generics: syn::Generics,
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
}

/*#[derive(Default, FromMeta)]
#[darling(default)]
struct StagesMeta {
    all_graphics: Flag,
    compute: Flag,
    vertex: Flag,
    fragment: Flag,
    geometry: Flag,
    tessellation_control: Flag,
    tessellation_evaluation: Flag,
}*/

#[derive(Default, FromMeta)]
struct RuntimeArrayMeta {
    max_count: u32,
}

/*#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
}*/

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

/*fn generate_shader_access_mask(stages: &SpannedValue<StagesMeta>) -> TokenStream {
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
}*/

struct Binding<'a> {
    field: &'a syn::Field,
    field_index: usize,
    binding: u32,
}

pub fn derive(input: proc_macro::TokenStream) -> Result<TokenStream, syn::Error> {
    let derive_input: syn::DeriveInput = syn::parse(input)?;

    // check for struct
    let fields = match derive_input.data {
        Data::Struct(ref struct_data) => &struct_data.fields,
        _ => {
            return Err(syn::Error::new(
                derive_input.span(),
                "`ShaderArguments` can only be derived on structs",
            ));
        }
    };
    // check for `#[repr(C)]`
    ensure_repr_c_derive_input(&derive_input)?;

    // parse struct-level `#[arguments(...)]` attribs.
    let s: ArgumentsStruct =
        <ArgumentsStruct as FromDeriveInput>::from_derive_input(&derive_input).unwrap();

    //let mut runtime_array_writes = Vec::new();
    let mut bindings = Vec::new();
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
                    bindings.push(Binding {
                        field,
                        field_index,
                        binding: attr.binding,
                    })
                }
            },
            Err(e) => {
                panic!("FIXME")
            }
        }
    }

    // --- Generate default uniform buffer struct ---
    let default_ubo_struct = quote! {
        #[repr(C)]
        struct DefaultUniformBuffer {
            #(#default_uniform_buffer_fields,)*
        }
    };

    // --- Generate descriptor set layout bindings ---
    let descriptor_set_layout_bindings: Vec<_> = bindings
        .iter()
        .map(|b| {
            let ty = &b.field.ty;
            let binding = b.binding;
            if binding == 0 && !default_uniform_buffer_fields.is_empty() {
                // binding #0 is reserved for the default uniform buffer is there's one: ensure it's not used
                syn::Error::new(
                    b.field.span(),
                    "Binding number 0 is reserved for inline uniforms",
                )
                .into_compile_error()
            } else {
                quote! {
                    #CRATE::vk::DescriptorSetLayoutBinding {
                        binding           : #binding,
                        stage_flags       : <#ty as #CRATE::DescriptorBinding>::STAGE_FLAGS,
                        descriptor_type   : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_TYPE,
                        descriptor_count  : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT,
                        immutable_samplers: ::std::ptr::null()
                    }
                }
            }
        })
        .collect();

    // --- Generate update template entries ---
    let descriptor_update_template_entries: Vec<_> = bindings
        .iter()
        .map(|b| {
            let binding = b.binding;
            let ty = &b.field.ty;
            quote! {
                #CRATE::vk::DescriptorUpdateTemplateEntry {
                    dst_binding       : #binding,
                    dst_array_element : 0,
                    descriptor_count  : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_COUNT,
                    descriptor_type   : <#ty as #CRATE::DescriptorBinding>::DESCRIPTOR_TYPE,
                    offset            : <#ty as #CRATE::DescriptorBinding>::UPDATE_OFFSET,
                    stride            : <#ty as #CRATE::DescriptorBinding>::UPDATE_STRIDE
                }
            }
        })
        .collect();

    // --- Descriptor update code ---
    let descriptor_prepare_update_statements: Vec<_> = bindings
        .iter()
        .map(|b| {
            if let Some(ref ident) = b.field.ident {
                quote! { #CRATE::DescriptorBinding::prepare_update(&mut self.#ident, ctx) }
            } else {
                let index = syn::Index::from(b.field_index);
                quote! { #CRATE::DescriptorBinding::prepare_update(&mut self.#index, ctx) }
            }
        })
        .collect();

    let struct_name = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();

    let q = quote! {
        impl #impl_generics #CRATE::ShaderArguments for #struct_name #ty_generics #where_clause {

            fn unique_type_id(&self) -> Option<::std::any::TypeId> {
                Some(::std::any::TypeId::of::<Self>())
            }

            fn get_descriptor_set_layout_bindings(&self) -> &[#CRATE::vk::DescriptorSetLayoutBinding]
            {
                const BINDINGS: &'static [#CRATE::vk::DescriptorSetLayoutBinding] = &[ #(#descriptor_set_layout_bindings,)* ];
                BINDINGS
            }

            fn get_descriptor_set_update_template_entries(
                &self,
            ) -> Option<&[#CRATE::vk::DescriptorUpdateTemplateEntry]>
            {
                const UPDATE_TEMPLATE_ENTRIES: &'static [#CRATE::vk::DescriptorUpdateTemplateEntry] = &[#(#descriptor_update_template_entries,)*];
                Some(UPDATE_TEMPLATE_ENTRIES)
            }

            unsafe fn update_descriptor_set(
                &mut self,
                ctx: &mut #CRATE::RecordingContext,
                set: #CRATE::vk::DescriptorSet,
                update_template: Option<#CRATE::vk::DescriptorUpdateTemplate>)
            {
                #(#descriptor_prepare_update_statements)*
                let device = ctx.vulkan_device();
                // update with template
                if let Some(update_template) = update_template {
                    device.update_descriptor_set_with_template(set, update_template, self as *const _ as *const ::std::ffi::c_void);
                }

                // TODO make default buffer struct
                // TODO write descriptor of default buffer
                // TODO write runtime-sized arrays
            }
        }
    };

    Ok(q)
}
