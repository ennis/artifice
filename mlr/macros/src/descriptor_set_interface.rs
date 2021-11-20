use crate::{ensure_repr_c, generate_field_offsets_and_sizes, FieldList, CRATE};
use darling::{
    util::{Flag, SpannedValue},
    FromDeriveInput, FromField, FromMeta,
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;

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
    //#[darling(default)]
    //array: SpannedValue<Flag>,
    #[darling(default)]
    stages: Option<SpannedValue<StagesMeta>>,
    #[darling(default)]
    uniform: Flag,
    #[darling(default)]
    sampler: Flag,
    #[darling(default)]
    sampled_image: Flag,
    #[darling(default)]
    storage_image: Flag,
    #[darling(default)]
    uniform_buffer: Flag,
    #[darling(default)]
    uniform_buffer_dynamic: Flag,
    #[darling(default)]
    storage_buffer: Flag,
    #[darling(default)]
    storage_buffer_dynamic: Flag,
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
    kind: ArgumentKind,
    descriptor_type: TokenStream, // VkDescriptorType
    stages: TokenStream,          // expr of type VkShaderStageFlags
}

impl<'a> Binding<'a> {
    fn parse(
        field: &'a syn::Field,
        field_index: usize,
        kind: ArgumentKind,
        attr: &ArgumentAttr,
    ) -> Binding<'a> {
        assert_ne!(kind, ArgumentKind::Uniform);

        // shader stage access mask
        let stages = if let Some(stages) = &attr.stages {
            generate_shader_access_mask(stages)
        } else {
            quote! { #CRATE::vk::ShaderStageFlags::from_raw(#CRATE::vk::ShaderStageFlags::ALL_GRAPHICS.as_raw() | #G::vk::ShaderStageFlags::COMPUTE.as_raw()) }
        };

        let binding = attr.binding;
        let descriptor_type = kind.descriptor_type().unwrap();

        Binding {
            field,
            field_index,
            binding,
            kind,
            descriptor_type,
            stages,
        }
    }

    fn descriptor_write_type(&self) -> DescriptorWriteType {
        match self.kind {
            ArgumentKind::Sampler | ArgumentKind::SampledImage | ArgumentKind::StorageImage => {
                DescriptorWriteType::Image
            }
            ArgumentKind::UniformBuffer
            | ArgumentKind::UniformBufferDynamic
            | ArgumentKind::StorageBuffer
            | ArgumentKind::StorageBufferDynamic => DescriptorWriteType::Buffer,
            _ => unreachable!("unexpected argument kind"),
        }
    }

    fn generate_vk_descriptor_set_layout_binding(&self) -> TokenStream {
        let ty = &self.field.ty;
        let binding = self.binding;
        let descriptor_type = &self.descriptor_type;
        let stages = &self.stages;

        quote! {
            #CRATE::vk::DescriptorSetLayoutBinding {
                binding           : #binding,
                stage_flags       : #stages,
                descriptor_type   : #descriptor_type,
                descriptor_count  : <#ty as #CRATE::DescriptorSource>::ARRAY_SIZE,
                immutable_samplers: ::std::ptr::null()
            }
        }
    }

    fn generate_vk_descriptor_update_template_entry(&self) -> TokenStream {
        todo!()
    }
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
        let mut default_uniform_buffer_fields = Vec::new();

        // parse field bindings
        for (field_index, f) in fields.iter().enumerate() {
            match <ArgumentAttr as FromField>::from_field(f) {
                Ok(attr) => {
                    let attr: &ArgumentAttr = &attr;

                    // argument type
                    let mut num_kinds = 0;
                    let mut kind = ArgumentKind::Uniform;
                    if attr.sampler.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::Sampler;
                    }
                    if attr.sampled_image.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::SampledImage;
                    }
                    if attr.storage_image.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::StorageImage;
                    }
                    if attr.uniform_buffer.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::UniformBuffer;
                    }
                    if attr.uniform_buffer_dynamic.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::UniformBufferDynamic;
                    }
                    if attr.storage_buffer.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::StorageBuffer;
                    }
                    if attr.storage_buffer_dynamic.is_some() {
                        num_kinds += 1;
                        kind = ArgumentKind::StorageBufferDynamic;
                    }

                    static ARGUMENT_KIND_EXPECTED_TOKENS: &str = "Expected exactly one of `sampled_image`, `storage_image`, `uniform_buffer`, `uniform_buffer_dynamic`, `storage_buffer`, `storage_buffer_dynamic`, `uniform`.";
                    if num_kinds > 1 {
                        f.span()
                            .unwrap()
                            .error("More than one argument kind specified.")
                            .note(ARGUMENT_KIND_EXPECTED_TOKENS)
                            .emit();
                        continue;
                    };

                    match kind {
                        ArgumentKind::Uniform => {
                            // this is an inline uniform
                            default_uniform_buffer_fields.push(f);
                        }
                        _ => {
                            // this is an actual binding
                            bindings.push(Binding::parse(f, field_index, kind, attr))
                        }
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

    /*fn generate_descriptor_holder(&self) -> TokenStream {
        let descriptors: Vec<_> = self.bindings.iter().map(|b| {
            let name = b.field.ident.as_ref().unwrap();
            let ty = &b.field.ty;
            quote! { #name: [<#ty as #CRATE::DescriptorSource>::DescriptorWriteType }
        }).collect();

        quote! {
            #[repr(C)]
            struct DescriptorHolder {
                #(#descriptors,)*
            }
        }
    }*/

    fn generate_vk_descriptor_update_template_entries(&self) -> TokenStream {
        let mut descriptor_write_offset_consts = Vec::with_capacity(self.binding.len());
        let mut descriptor_update_fields = Vec::with_capacity(self.binding.len());
        let mut update_template_entries = Vec::with_capacity(self.binding.len());

        for (i, b) in self.bindings.iter().enumerate() {
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
                #CRATE::vk::DescriptorUpdateTemplateEntry {
                    dst_binding       : #binding,
                    dst_array_element : 0,
                    descriptor_count  : <#field_ty as #CRATE::DescriptorSource>::ARRAY_SIZE,
                    descriptor_type   : #descriptor_type,
                    offset            : #offset_0,
                    stride            : #size
                }
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

pub fn generate(derive_input: &syn::DeriveInput, fields: &FieldList) -> TokenStream {
    let s: ArgumentsStruct =
        <ArgumentsStruct as FromDeriveInput>::from_derive_input(derive_input).unwrap();
    let struct_name = &s.ident;
    let (impl_generics, ty_generics, where_clause) = s.generics.split_for_impl();

    if let Err(e) = ensure_repr_c("ShaderArguments", derive_input) {
        return e;
    }

    let field_offsets_sizes = generate_field_offsets_and_sizes(derive_input);

    let mut bindings = Vec::new();
    let mut update_template_entries = Vec::new();
    let mut runtime_array_writes = Vec::new();
    let mut has_default_uniform_buffer = false;
    // contents of the default uniform buffer (one is generated when you put data inline)

    for (i_field, f) in fields.iter().enumerate() {
        let ty = &f.ty;
        let name = f.ident.as_ref().unwrap();

        match <ArgumentAttr as FromField>::from_field(f) {
            Ok(attr) => {
                let attr: &ArgumentAttr = &attr;

                // shader stage access mask
                let stages = if let Some(stages) = &attr.stages {
                    generate_shader_access_mask(stages)
                } else {
                    quote! { #CRATE::vk::ShaderStageFlags::from_raw(#CRATE::vk::ShaderStageFlags::ALL_GRAPHICS.as_raw() | #G::vk::ShaderStageFlags::COMPUTE.as_raw()) }
                };

                // infer descriptor type
                let binding = attr.binding;
                let kind = if let Some(kind) = attr.get_argument_kind() {
                    kind
                } else {
                    continue;
                };

                match kind {
                    ArgumentKind::Uniform => {
                        default_uniform_buffer_fields.push(f);
                        continue;
                    }
                    _ => {}
                }

                let descriptor_type = kind.descriptor_type().unwrap();

                // add binding entry
                bindings.push(Binding {
                    field: f,
                    field_index: 0,
                    binding,
                    kind,
                    descriptor_type,
                    stages,
                });

                /*if let Some(runtime_array) = &attr.runtime_array {
                    let max_count = runtime_array.max_count;
                    binding_infos.push(quote! {
                        #G::DescriptorSetLayoutBindingInfo {
                            binding           : #binding,
                            stage_flags       : #stages,
                            descriptor_type   : #descriptor_type_tokens,
                            descriptor_count  : #max_count,
                            //immutable_samplers: [#G::vk::Sampler::null(); 16]
                        }
                    });
                    let p_image_info = match descriptor_type {
                        DescriptorType::Sampler
                        | DescriptorType::SampledImage
                        | DescriptorType::StorageImage => {
                            quote! { self.#name.as_ptr() } // TODO use DescriptorSource?
                        }
                        _ => quote! {::std::ptr::null()},
                    };
                    let p_buffer_info = match descriptor_type {
                        DescriptorType::UniformBuffer
                        | DescriptorType::UniformBufferDynamic
                        | DescriptorType::StorageBuffer
                        | DescriptorType::StorageBufferDynamic => {
                            quote! { self.#name.as_ptr() } // TODO use DescriptorSource?
                        }
                        _ => quote! {::std::ptr::null()},
                    };
                    let p_texel_buffer_view = quote! {::std::ptr::null()};

                    runtime_array_writes.push(quote! {
                        #G::vk::WriteDescriptorSet {
                            dst_set             : set,   // hardcoded
                            dst_binding         : #binding,
                            dst_array_element   : 0,
                            descriptor_count    : self.#name.len() as u32,  // TODO use DescriptorSource?
                            descriptor_type     : #descriptor_type_tokens,
                            p_image_info        : #p_image_info,
                            p_buffer_info       : #p_buffer_info,
                            p_texel_buffer_view : #p_texel_buffer_view,
                            .. Default::default()
                        }
                    });
                } else */

                /*{
                    binding_infos.push(quote! {
                        #CRATE::DescriptorSetLayoutBindingInfo {
                            binding           : #binding,
                            stage_flags       : #stages,
                            descriptor_type   : #descriptor_type_tokens,
                            descriptor_count  : <#ty as #CRATE::DescriptorSource>::ARRAY_SIZE,
                            //immutable_samplers: [#G::vk::Sampler::null(); 16]
                        }
                    });
                    let offset_ident = &field_offsets_sizes.offsets[i_field].ident;
                    let size_ident = &field_offsets_sizes.sizes[i_field].ident;
                    update_template_entries.push(quote! {
                        #CRATE::vk::DescriptorUpdateTemplateEntry {
                            dst_binding       : #binding,
                            dst_array_element : 0,
                            descriptor_count  : <#ty as #CRATE::DescriptorSource>::ARRAY_SIZE,
                            descriptor_type   : #descriptor_type_tokens,
                            offset            : Self::#offset_ident,
                            stride            : Self::#size_ident
                        }
                    });
                }*/
            }
            Err(e) => {
                e.write_errors();
            }
        }
    }

    let field_offsets_sizes_impl = field_offsets_sizes.impl_block;

    let q = quote! {
        #field_offsets_sizes_impl

        impl #impl_generics #G::DescriptorSetInterface for #struct_name #ty_generics #where_clause {
            const LAYOUT: &'static [#G::DescriptorSetLayoutBindingInfo] = &[ #(#binding_infos,)* ];
            const UPDATE_TEMPLATE_ENTRIES: &'static [#G::vk::DescriptorUpdateTemplateEntry] = &[#(#update_template_entries,)*];

            fn get_or_init_layout(init: impl FnOnce() -> #G::DescriptorSetAllocatorId) -> #G::DescriptorSetAllocatorId {
                static LAYOUT_ID: #G::internal::OnceCell<#G::DescriptorSetAllocatorId> = #G::internal::OnceCell::new();
                *LAYOUT_ID.get_or_init(init)
            }

            unsafe fn update_descriptors(
                &self,
                device: &#G::ash::Device,
                set: #G::vk::DescriptorSet,
                update_template: #G::vk::DescriptorUpdateTemplate)
            {
                use #G::ash::version::DeviceV1_0;
                use #G::ash::version::DeviceV1_1;
                if !Self::UPDATE_TEMPLATE_ENTRIES.is_empty() {
                    device.update_descriptor_set_with_template(set, update_template, self as *const _ as *const ::std::ffi::c_void);
                }
                // write runtime-sized arrays
                let descriptor_writes = &[#(#runtime_array_writes,)*];
                if !descriptor_writes.is_empty() {
                    device.update_descriptor_sets(descriptor_writes, &[]);
                }
            }
        }
    };
    q
}
