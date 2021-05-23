use crate::{
    context::{
        is_write_access,
        pass::{
            Pass, PassCommands, ResourceAccess, SemaphoreSignal, SemaphoreSignalKind,
            SemaphoreWait, SemaphoreWaitKind,
        },
        BufferId, BufferInfo, ImageId, ResourceAccessDetails, ResourceId, ResourceKind,
        ResourceTrackingInfo, TypedBufferInfo,
        CommandContext, FrameInFlight, FrameSerialNumber, GpuFuture, QueueSerialNumbers,
        SubmissionNumber,
    },
    vk,
    vk::Handle,
    BufferResourceCreateInfo, Context, ImageInfo, ImageResourceCreateInfo, ResourceMemoryInfo,
    MAX_QUEUES,
};
use slotmap::{Key, SecondaryMap};
use std::{
    cell::{RefCell, RefMut},
    mem, ptr,
};
use tracing::trace_span;
use crate::context::{compute_reachability, AllocationRequirements, Resource, ResourceOwnership};
use crate::ash::version::DeviceV1_0;
use crate::swapchain::SwapchainImage;

type TemporarySet = std::collections::BTreeSet<ResourceId>;

// TODO this is here only for convenience, could be removed
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum AccessType {
    /// Vertex attribute read in the vertex input stage
    VertexAttributeRead,
    /// Index read in the vertex input stage
    IndexRead,

    /// Uniform buffer read in the vertex shader stage.
    VertexShaderReadUniformBuffer,
    /// Uniform buffer read in the fragment shader stage.
    FragmentShaderReadUniformBuffer,
    /// Uniform buffer read in the geometry shader stage.
    GeometryShaderReadUniformBuffer,
    /// Uniform buffer read in the tessellation control shader stage.
    TessControlShaderReadUniformBuffer,
    /// Uniform buffer read in the tessellation evaluation shader stage.
    TessEvalShaderReadUniformBuffer,
    /// Uniform buffer read in the compute shader stage.
    ComputeShaderReadUniformBuffer,
    /// Uniform buffer read in any shader stage.
    AnyShaderReadUniformBuffer,

    /// Sampled image read in the vertex shader stage.
    VertexShaderReadSampledImage,
    /// Sampled image read in the fragment shader stage.
    FragmentShaderReadSampledImage,
    /// Sampled image read in the geometry shader stage.
    GeometryShaderReadSampledImage,
    /// Sampled image read in the tessellation control shader stage.
    TessControlShaderReadSampledImage,
    /// Sampled image read in the tessellation evaluation shader stage.
    TessEvalShaderReadSampledImage,
    /// Sampled image read in the compute shader stage.
    ComputeShaderReadSampledImage,
    /// Sampled image read in any shader stage.
    AnyShaderReadSampledImage,

    /// Any read other than uniform buffers & sampled images in the vertex shader stage.
    VertexShaderReadOther,
    /// Any read other than uniform buffers & sampled images in the fragment shader stage.
    FragmentShaderReadOther,
    /// Any read other than uniform buffers & sampled images in the geometry shader stage.
    GeometryShaderReadOther,
    /// Any read other than uniform buffers & sampled images in the tessellation control shader stage.
    TessControlShaderReadOther,
    /// Any read other than uniform buffers & sampled images in the tessellation evaluation shader stage.
    TessEvalShaderReadOther,
    /// Any read other than uniform buffers & sampled images in the compute shader stage.
    ComputeShaderReadOther,
    /// Any read other than uniform buffers & sampled images in any shader stage.
    AnyShaderReadOther,

    /// Any write in the vertex shader stage.
    VertexShaderWrite,
    /// Any write in the fragment shader stage.
    FragmentShaderWrite,
    /// Any write in the geometry shader stage.
    GeometryShaderWrite,
    /// Any write in the tessellation control shader stage.
    TessControlShaderWrite,
    /// Any write in the tessellation evaluation shader stage.
    TessEvalShaderWrite,
    /// Any write in the compute shader stage.
    ComputeShaderWrite,
    /// Any write in any shader stage.
    AnyShaderWrite,

    /// Read transfer source
    TransferRead,
    /// Written transfer destination
    TransferWrite,

    /// Written color attachment
    ColorAttachmentWrite,
    /// Read and written color attachment.
    ColorAttachmentReadWrite,
    /// Read-only depth attachment
    DepthStencilAttachmentRead,
    /// Written depth attachment
    DepthStencilAttachmentWrite,
    /// Read/written depth attachment
    DepthStencilAttachmentReadWrite,
}

// FIXME it would be more precise to specify each shader stage instead of ALL_COMMANDS, but
// then we need to check that the device actually supports the stages in question.
// VK_KHR_synchronization2 has VK_PIPELINE_STAGE_2_PRE_RASTERIZATION_SHADERS_BIT_KHR that can be used to this effect.
const ANY_SHADER_STAGE: vk::PipelineStageFlags = vk::PipelineStageFlags::ALL_COMMANDS;

impl AccessType {
    pub fn to_access_info(&self) -> AccessTypeInfo {
        match *self {
            AccessType::VertexAttributeRead => AccessTypeInfo {
                access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
                stage_mask: vk::PipelineStageFlags::VERTEX_INPUT,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::IndexRead => AccessTypeInfo {
                access_mask: vk::AccessFlags::INDEX_READ,
                stage_mask: vk::PipelineStageFlags::VERTEX_INPUT,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::VertexShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: vk::PipelineStageFlags::VERTEX_SHADER,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::FragmentShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::GeometryShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: vk::PipelineStageFlags::GEOMETRY_SHADER,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::TessControlShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::TessEvalShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::ComputeShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: vk::PipelineStageFlags::COMPUTE_SHADER,
                layout: vk::ImageLayout::UNDEFINED,
            },
            AccessType::AnyShaderReadUniformBuffer => AccessTypeInfo {
                access_mask: vk::AccessFlags::UNIFORM_READ,
                stage_mask: ANY_SHADER_STAGE,
                layout: vk::ImageLayout::UNDEFINED,
            },

            AccessType::VertexShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::VERTEX_SHADER,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            AccessType::FragmentShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            AccessType::GeometryShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::GEOMETRY_SHADER,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            AccessType::TessControlShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            AccessType::TessEvalShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            AccessType::ComputeShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::COMPUTE_SHADER,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            AccessType::AnyShaderReadSampledImage => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: ANY_SHADER_STAGE,
                layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },

            AccessType::VertexShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::VERTEX_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::FragmentShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::GeometryShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::GEOMETRY_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::TessControlShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::TessEvalShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::ComputeShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: vk::PipelineStageFlags::COMPUTE_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::AnyShaderReadOther => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_READ,
                stage_mask: ANY_SHADER_STAGE,
                layout: vk::ImageLayout::GENERAL,
            },

            AccessType::VertexShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: vk::PipelineStageFlags::VERTEX_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::FragmentShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::GeometryShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: vk::PipelineStageFlags::GEOMETRY_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::TessControlShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::TessEvalShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::ComputeShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: vk::PipelineStageFlags::COMPUTE_SHADER,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::AnyShaderWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::SHADER_WRITE,
                stage_mask: ANY_SHADER_STAGE,
                layout: vk::ImageLayout::GENERAL,
            },
            AccessType::TransferRead => AccessTypeInfo {
                access_mask: vk::AccessFlags::TRANSFER_READ,
                stage_mask: vk::PipelineStageFlags::TRANSFER,
                layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            },
            AccessType::TransferWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::TRANSFER_WRITE,
                stage_mask: vk::PipelineStageFlags::TRANSFER,
                layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            },

            AccessType::ColorAttachmentWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            },

            AccessType::ColorAttachmentReadWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            },
            AccessType::DepthStencilAttachmentWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            },
            AccessType::DepthStencilAttachmentReadWrite => AccessTypeInfo {
                access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            },
            AccessType::DepthStencilAttachmentRead => AccessTypeInfo {
                access_mask: vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
                stage_mask: vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                    | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                layout: vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct AccessTypeInfo {
    pub access_mask: vk::AccessFlags,
    pub stage_mask: vk::PipelineStageFlags,
    pub layout: vk::ImageLayout,
}

/*
/// Adds an execution dependency between a source and destination pass, identified by their submission numbers.
/// Returns whether the dependency is realized with a semaphore or not.
fn add_execution_dependency(
    passes: &mut [Pass],
    base_serial: u64,
    src_snn: SubmissionNumber,
    dst: &mut Pass,
    dst_stage_mask: vk::PipelineStageFlags,
) -> &mut Barrier {
    let src = get_pass_mut(base_serial, passes, src_snn);

    if let Some(src) = src {
        // --- Intra-frame synchronization
        let r = if src_snn.queue() != dst.snn.queue() {
            // cross-queue dependency w/ timeline semaphore
            src.signal_after = true;
            let q = src_snn.queue();
            dst.wait_before = true;
            dst.wait_serials[q] = dst.wait_serials[q].max(src_snn.serial());
            dst.wait_dst_stages[q as usize] |= dst_stage_mask;
            true
        } else {
            // same-queue dependency, a pipeline barrier is sufficient
            dst.src_stage_mask |= src.output_stage_mask;
            false
        };

        dst.preds.push(src.frame_index);
        src.succs.push(dst.frame_index);
        r
    } else {
        // --- Inter-frame synchronization w/ timeline semaphore
        let q = src_snn.queue();
        dst.wait_before = true;
        dst.wait_serials[q] = dst.wait_serials[q].max(src_snn.serial());
        dst.wait_dst_stages[q as usize] |= dst_stage_mask;
        true
    }
}*/

/*/// Helper to find the pass given a submission number.
fn get_pass_mut<'a, 'b>(
    start_serial: u64,
    passes: &'a mut [Pass<'b>],
    snn: SubmissionNumber,
) -> Option<&'a mut Pass<'b>> {
    if snn.serial() <= start_serial {
        None
    } else {
        let pass_index = (snn.serial() - start_serial - 1) as usize;
        Some(&mut passes[pass_index])
    }
}*/

/// Builder object for passes.
pub struct PassBuilder<'a, 'frame> {
    frame: &'frame mut FrameInner<'a>,
    pass: Pass<'a>,
    _span: tracing::span::EnteredSpan,
}

impl<'a, 'frame> PassBuilder<'a, 'frame> {
    /// Adds a semaphore wait operation: the pass will first wait for the specified semaphore to be signalled
    /// before starting.
    pub fn add_external_semaphore_wait(
        &mut self,
        semaphore: vk::Semaphore,
        dst_stage: vk::PipelineStageFlags,
        wait_kind: SemaphoreWaitKind,
    ) {
        self.pass.external_semaphore_waits.push(SemaphoreWait {
            semaphore,
            owned: false,
            dst_stage,
            wait_kind,
        })
    }

    /// Adds a semaphore signal operation: when finished, the pass will signal the specified semaphore.
    pub fn add_external_semaphore_signal(
        &mut self,
        semaphore: vk::Semaphore,
        signal_kind: SemaphoreSignalKind,
    ) {
        self.pass.external_semaphore_signals.push(SemaphoreSignal {
            semaphore,
            signal_kind,
        })
    }

    pub fn register_image_access(&mut self, id: ImageId, access_type: AccessType) {
        let AccessTypeInfo {
            access_mask,
            stage_mask,
            layout,
        } = access_type.to_access_info();
        self.register_image_access_2(id, access_mask, stage_mask, layout, layout);
    }

    /// Registers an image access made by this pass.
    pub fn register_image_access_2(
        &mut self,
        id: ImageId,
        access_mask: vk::AccessFlags,
        stage_mask: vk::PipelineStageFlags,
        initial_layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
    ) {
        self.frame.add_resource_dependency(
            &mut self.pass,
            id.0,
            &ResourceAccessDetails {
                initial_layout,
                final_layout,
                access_mask,
                stage_mask,
            },
        )
    }

    pub fn register_buffer_access(&mut self, id: BufferId, access_type: AccessType) {
        let AccessTypeInfo {
            access_mask,
            stage_mask,
            ..
        } = access_type.to_access_info();
        self.register_buffer_access_2(id, access_mask, stage_mask);
    }

    pub fn register_buffer_access_2(
        &mut self,
        id: BufferId,
        access_mask: vk::AccessFlags,
        stage_mask: vk::PipelineStageFlags,
    ) {
        self.frame.add_resource_dependency(
            &mut self.pass,
            id.0,
            &ResourceAccessDetails {
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::UNDEFINED,
                access_mask,
                stage_mask,
            },
        )
    }

    /// Sets the command handler for this pass.
    /// The handler will be called when building the command buffer, on batch submission.
    pub fn set_commands(
        &mut self,
        commands: impl FnOnce(&mut CommandContext, vk::CommandBuffer) + 'a,
    ) {
        self.pass.commands = Some(PassCommands::CommandBuffer(Box::new(commands)));
    }

    pub fn set_queue_commands(
        &mut self,
        commands: impl FnOnce(&mut CommandContext, vk::Queue) + 'a,
    ) {
        self.pass.commands = Some(PassCommands::Queue(Box::new(commands)));
    }
}

struct SyncDebugInfo {
    tracking: slotmap::SecondaryMap<ResourceId, ResourceTrackingInfo>,
    xq_sync_table: [QueueSerialNumbers; MAX_QUEUES],
}

impl SyncDebugInfo {
    fn new() -> SyncDebugInfo {
        SyncDebugInfo {
            tracking: Default::default(),
            xq_sync_table: Default::default(),
        }
    }
}

// ---------------------------------------------------------------------------------------
struct FrameInner<'a> {
    base_serial: u64,
    context: &'a mut Context,
    /// Map temporary index -> resource
    temporaries: Vec<ResourceId>,
    /// Set of all resources referenced in the frame
    temporary_set: TemporarySet,
    /// List of passes
    passes: Vec<Pass<'a>>,
    /// Serials to wait for before executing the frame.
    wait_init: QueueSerialNumbers,
    /// Cross-queue synchronization table
    /// TODO detailed description
    xq_sync_table: [QueueSerialNumbers; MAX_QUEUES],

    collect_sync_debug_info: bool,
    sync_debug_info: Vec<SyncDebugInfo>,
}

fn local_pass_index(serial: u64, frame_base_serial: u64) -> usize {
    assert!(serial > frame_base_serial);
    (serial - frame_base_serial - 1) as usize
}

impl<'a> FrameInner<'a> {
    /// Registers an access to a resource within the specified pass and updates the dependency graph.
    ///
    /// This is the meat of the automatic synchronization system: given the known state of the resources,
    /// this function infers the necessary execution barriers, memory barriers, and layout transitions,
    /// and updates the state of the resources.
    fn add_resource_dependency(
        &mut self,
        dst_pass: &mut Pass<'a>,
        id: ResourceId,
        access: &ResourceAccessDetails,
    ) {
        //------------------------
        // first, add the resource into the set of temporaries used within this frame
        let resource = self.context.resources.get_mut(id).unwrap();
        if self.temporary_set.insert(id) {
            self.temporaries.push(id);
        }

        // First, some definitions:
        // - current pass         : the pass which is accessing the resource, and for which we are registering a dependency
        // - current SN           : the SN of the current pass
        // - writer pass          : the pass that last wrote to the resource
        // - writer SN            : the SN of the writer pass
        // - input stage mask     : the pipeline stages that will access the resource in the current pass
        // - writer stage mask    : the pipeline stages that the writer
        // - availability barrier : the memory barrier that is in charge of making the writes available and visible to other stages
        //                          typically, the barrier is stored in the first reader
        //
        // 1. First, determine if we can do without any kind of synchronization. This is the case if:
        //      - the resource has no explicit binary semaphore to synchronize with
        //      - AND all previous writes are already visible
        //      - AND the resource doesn't need a layout transition
        //      -> If all of this is true, then skip to X
        // 2. Get or create the barrier
        //      The resulting barrier might be associated to the pass, or an existing one that comes before

        // If the resource has an associated semaphore, consume it.
        // For now, the only resources that have associated semaphores are swapchain images from the presentation engine.
        let semaphore = mem::take(&mut resource.tracking.wait_binary_semaphore);
        let has_external_semaphore = semaphore != vk::Semaphore::null();
        if has_external_semaphore {
            dst_pass.external_semaphore_waits.push(SemaphoreWait {
                semaphore,
                owned: true,
                dst_stage: vk::PipelineStageFlags::TOP_OF_PIPE, // FIXME maybe?
                wait_kind: SemaphoreWaitKind::Binary,
            });
        }

        //------------------------
        let need_layout_transition = resource.tracking.layout != access.initial_layout;

        // is the access a write? for synchronization purposes, layout transitions are the same thing as a write
        let is_write = is_write_access(access.access_mask) || need_layout_transition;

        // can we ensure that all previous writes are visible?
        // note: the visibility mask is only valid if this access and the last write is in the same queue
        // for cross-queue accesses, we never skip
        let writes_visible = resource.tracking.writer.queue() == dst_pass.snn.queue()
            && (resource
                .tracking
                .visibility_mask
                .contains(access.access_mask)
                || resource
                    .tracking
                    .visibility_mask
                    .contains(vk::AccessFlags::MEMORY_READ));

        // --- (1) skip to the end if no barrier is needed
        // No barrier is needed if we waited on an external semaphore, or all writes are visible and no layout transition is necessary

        if (!has_external_semaphore && !writes_visible) || need_layout_transition {
            let q = dst_pass.snn.queue() as usize;

            // Determine the "sources" of the dependency: i.e. the passes (identified by serials),
            // that we must synchronize with.
            //
            // If we're writing to the resource, and the resource is being read, we must wait for
            // all reads to complete, and thus synchronize with the readers.
            // Otherwise, if we're only reading from the resource, or if we're writing but there are no readers,
            // we must synchronize with the last writer (we can have multiple concurrent readers).
            //
            // Note that a resource can't have both a reader and a writer at the same time.
            let sync_sources = if is_write && resource.tracking.has_readers() {
                // Write-after-read dependency
                resource.tracking.readers
            } else {
                // Write-after-write, read-after-write
                QueueSerialNumbers::from_submission_number(resource.tracking.writer)
            };

            // from here, we have two possible methods of synchronization:
            // 1. semaphore signal/wait: this is straightforward, there's not a lot we can do
            // 2. pipeline barrier: we can choose **where** to put the barrier,
            //    we can put it anywhere between the source and the destination pass
            // -> if the source is just before, then use a pipeline barrier
            // -> if there's a gap, use an event?
            let base_serial = self.base_serial;

            // whether `sync_sources` identifies a single source pass in the queue that one we're
            // submitting on
            let single_local_source_in_same_queue = sync_sources
                .iter()
                .enumerate()
                .all(|(i, &sn)| (i != q && sn == 0) || (i == q && (sn == 0 || sn > base_serial)));

            if !single_local_source_in_same_queue {
                // Either:
                // - there are multiple sources across several queues
                // - the source is in a different queue
                // - the source is in an older frame
                // In those cases, a semaphore wait is necessary to synchronize.

                // go through each non-zero source
                for (iq, &sn) in sync_sources.iter().enumerate() {
                    if sn == 0 {
                        continue;
                    }

                    // look in the cross-queue sync table to see if there's already an execution dependency
                    // between the source (sn) and us.
                    if self.xq_sync_table[q].0[iq] >= sn {
                        // already synced
                        continue;
                    }

                    // we're adding a semaphore wait: update sync table
                    self.xq_sync_table[q].0[iq] = sn;

                    dst_pass.wait_serials.0[iq] = sn;
                    dst_pass.wait_dst_stages[iq] |= access.stage_mask;

                    if sn > self.base_serial {
                        // furthermore, if source and destination are in the same frame, add
                        // an edge to the depgraph (regardless of whether we added a semaphore or not)
                        let src_pass_index = local_pass_index(sn, self.base_serial);
                        //let dst_pass_index = dst_pass.frame_index;
                        let src_pass = &mut self.passes[src_pass_index];
                        //src_pass.succs.push(dst_pass_index);
                        //dst_pass.preds.push(src_pass_index);
                        src_pass.signal_queue_timelines = true;
                    }
                }
            } else {
                // There's only one source pass, which furthermore is on the same queue, and in the
                // same frame as the destination. In this case, we can use a pipeline barrier for
                // synchronization.

                let src_sn = sync_sources[q];
                let src_stage_mask = resource.tracking.stages;
                let dst_stage_mask = access.stage_mask;

                // sync dst=q, src=q
                if self.xq_sync_table[q][q] >= src_sn {
                    // if we're already synchronized with the source via a cross-queue (xq) wait
                    // (a.k.a. semaphore), we don't need to add a memory barrier.
                    // Note that layout transitions are handled separately, outside this condition.
                } else {
                    // not synced with a semaphore, see if there's already a pipeline barrier
                    // that ensures the execution dependency between the source (src_sn) and us

                    let local_src_index = local_pass_index(src_sn, self.base_serial);

                    // The question we ask ourselves now is: is there already an execution dependency,
                    // from the source pass, for the stages in `src_stage_mask`,
                    // to us (dst_pass), for the stages in `dst_stage_mask`,
                    // created by barriers in passes between the source and us?
                    //
                    // This is not easy to determine: to be perfectly accurate, we need to consider:
                    // - transitive dependencies: e.g. COMPUTE -> FRAGMENT and then FRAGMENT -> TRANSFER also creates a COMPUTE -> TRANSFER dependency
                    // - logically later and earlier stages: e.g. COMPUTE -> VERTEX also implies a COMPUTE -> FRAGMENT dependency
                    //
                    // For now, we just look for a pipeline barrier that directly contains the relevant stages
                    // (i.e. `barrier.src_stage_mask` contains `src_stage_mask`, and `barrier.dst_stage_mask` contains `dst_stage_mask`,
                    // ignoring transitive dependencies and any logical ordering between stages.
                    //
                    // The impact of this approximation is currently unknown.

                    // find a pipeline barrier that already takes care of our execution dependency
                    let barrier_pass = self.passes[local_src_index + 1..]
                        .iter_mut()
                        .find_map(|p| {
                            if p.snn.queue() == q
                                && p.src_stage_mask.contains(src_stage_mask)
                                && p.dst_stage_mask.contains(dst_stage_mask)
                            {
                                Some(p)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(dst_pass);

                    // add our stages to the execution dependency
                    barrier_pass.src_stage_mask |= src_stage_mask;
                    barrier_pass.dst_stage_mask |= dst_stage_mask;

                    // now deal with the memory dependency
                    match &resource.kind {
                        ResourceKind::Image(img) => {
                            let mb = barrier_pass
                                .get_or_create_image_memory_barrier(img.handle, img.format);
                            mb.src_access_mask |= resource.tracking.availability_mask;
                            mb.dst_access_mask |= access.access_mask;
                            // Also specify the layout transition here.
                            // This is redundant with the code after that handles the layout transition,
                            // but we might not always go through here when a layout transition is necessary.
                            // With Sync2, just set these to UNDEFINED.
                            mb.old_layout = resource.tracking.layout;
                            mb.new_layout = access.initial_layout;
                        }
                        ResourceKind::Buffer(buf) => {
                            let mb = barrier_pass.get_or_create_buffer_memory_barrier(buf.handle);
                            mb.src_access_mask |= resource.tracking.availability_mask;
                            mb.dst_access_mask |= access.access_mask;
                        }
                    }

                    // this memory dependency makes all writes on the resource available, and
                    // visible to the types specified in `access.access_mask`
                    resource.tracking.availability_mask = vk::AccessFlags::empty();
                    resource.tracking.visibility_mask |= access.access_mask;
                }
            }

            // layout transitions
            if need_layout_transition {
                let image = resource.image();
                let mb = dst_pass.get_or_create_image_memory_barrier(image.handle, image.format);
                mb.old_layout = resource.tracking.layout;
                mb.new_layout = access.initial_layout;
                resource.tracking.layout = access.final_layout;
            }
        }

        if is_write_access(access.access_mask) {
            // we're writing to the resource, so reset visibility...
            resource.tracking.visibility_mask = vk::AccessFlags::empty();
            // ... but signal that there is data to be made available for this resource.
            resource.tracking.availability_mask |= access.access_mask;
        }

        // update output stage
        // FIXME I have doubts about this code
        if is_write {
            resource.tracking.stages = access.stage_mask;
            resource.tracking.clear_readers();
            resource.tracking.writer = dst_pass.snn;
        } else {
            // update the resource readers
            resource.tracking.readers = resource.tracking.readers.join_serial(dst_pass.snn);
        }

        // record the access in the pass
        dst_pass.accesses.push(ResourceAccess {
            id,
            access_mask: access.access_mask,
        });
    }

    fn push_pass(&mut self, pass: Pass<'a>) {
        self.passes.push(pass);

        if self.collect_sync_debug_info {
            let mut info = SyncDebugInfo::new();
            // current resource tracking info
            for (id, r) in self.context.resources.iter() {
                info.tracking.insert(id, r.tracking);
            }
            // current sync table
            info.xq_sync_table = self.xq_sync_table;
            self.sync_debug_info.push(info);
        }
    }
}


fn allocate_memory_for_transients(
    context: &Context,
    base_serial: u64,
    passes: &[Pass],
    temporaries: &[ResourceId]) -> Vec<vk_mem::Allocation>
{
    let _span = trace_span!("allocate_memory_for_transients").entered();

    #[derive(Copy, Clone, Debug)]
    struct AllocIndex {
        index: usize,
        dead_and_recycled: bool,
    }

    let reachability = compute_reachability(&passes);

    // alloc index -> alloc requirements
    let mut requirements: Vec<AllocationRequirements> = Vec::new();
    // resource id -> allocation mapping (index+state)
    let mut alloc_map: SecondaryMap<ResourceId, AllocIndex> = SecondaryMap::new();

    fn get_allocation_requirements(resource: &Resource) -> Option<AllocationRequirements> {
        match &resource.ownership {
            ResourceOwnership::Referenced => {
                // skip non-owned resources
                None
            }
            ResourceOwnership::Owned { requirements, allocation } => {
                if allocation.is_some() {
                    // skip already allocated resources
                    None
                } else {
                    Some(*requirements)
                }
            }
        }
    }

    for &dst_id in temporaries.iter() {
        // SRC = the resource we want to alias with
        // DST = the resource we are allocating
        let dst = context.resources.get(dst_id).unwrap();
        let dst_req = if let Some(req) = get_allocation_requirements(dst) { req } else { continue };

        // try to find a suitable resource to alias with (the "source")
        let mut aliased = false;
        'alias: for (src_id, src) in context.resources.iter() {
            if src_id == dst_id {
                // don't alias with the same resource...
                continue;
            }

            // skip if not aliasable
            let _src_req  = if let Some(req) = get_allocation_requirements(src) { req } else { continue };

            let mut alloc_state =
                if let Some(alloc_state) = alloc_map.get_mut(src_id) {
                    // skip if the resource is already dead, and its memory was already reused
                    if alloc_state.dead_and_recycled {
                        continue;
                    }
                    alloc_state
                } else {
                    // skip if not allocated yet
                    continue;
                };

            let src_first_access = src.tracking.first_access.serial();

            // To re-use the memory of SRC in DST, SRC must be _dead_ before the first use of DST.
            // A resource is dead from the point of view of a pass if this pass has an execution
            // dependency on all last readers and writers of the resource.
            for &reader in src.tracking.readers.iter() {
                if reader != 0 &&
                    (reader >= src_first_access
                        || !reachability.is_reachable(
                        local_pass_index(reader, base_serial),
                        local_pass_index(src_first_access, base_serial))) {
                    continue 'alias;
                }
            }

            let writer = src.tracking.writer.serial();
            if writer != 0
                && (writer >= src_first_access
                || !reachability.is_reachable(local_pass_index(writer, base_serial), local_pass_index(src_first_access, base_serial)))
            {
                continue;
            }

            // if we reach here, then SRC is dead, and from a synchronization point of view
            // the resources may alias. However, we now need to check that the allocation
            // requirements of the two resources can be made compatible.
            let dead_alloc = &mut requirements[alloc_state.index];
            if !dead_alloc.try_adjust(&dst_req) {
                // the memory requirements of the two resources cannot be made compatible
                continue;
            }

            // success: the two resources may alias, and the memory requirements have been adjusted
            // now update the allocation map
            let index = alloc_state.index;
            alloc_state.dead_and_recycled = true;
            alloc_map.insert(
                dst_id,
                AllocIndex {
                    index,
                    dead_and_recycled: false,
                },
            );

            aliased = true;
            break;
        }

        if !aliased {
            // we could not alias with any existing resource, so create a new allocation for the resource
            let index = requirements.len();
            requirements.push(dst_req);
            alloc_map.insert(
                dst_id,
                AllocIndex {
                    index,
                    dead_and_recycled: false,
                },
            );
        }
    }

    // now allocate device memory
    let mut allocations = Vec::with_capacity(requirements.len());
    let mut allocation_infos = Vec::with_capacity(requirements.len());

    for alloc_req in requirements.iter() {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            ..Default::default()
        };
        let (allocation, allocation_info) = context
            .device
            .allocator
            .allocate_memory(&alloc_req.mem_req, &allocation_create_info)
            .expect("failed to allocate device memory");
        allocations.push(allocation);
        allocation_infos.push(allocation_info);
    }

    // and bind them to the resources
    for (id, resource) in context.resources.iter()
    {
        if let Some(alloc_index) = alloc_map.get(id) {
            let alloc_info = &allocation_infos[alloc_index.index];
            match &resource.kind {
                ResourceKind::Image(img) => unsafe {
                    context.device.device
                        .bind_image_memory(
                            img.handle,
                            alloc_info.get_device_memory(),
                            alloc_info.get_offset() as u64,
                        )
                        .unwrap();
                },
                ResourceKind::Buffer(buf) => unsafe {
                    context.device.device
                        .bind_buffer_memory(
                            buf.handle,
                            alloc_info.get_device_memory(),
                            alloc_info.get_offset() as u64,
                        )
                        .unwrap();
                },
            }
        }
    }

    allocations
}

#[derive(Copy, Clone, Debug)]
pub struct FrameCreateInfo {
    pub happens_after: GpuFuture,
    pub collect_debug_info: bool,
}

impl Default for FrameCreateInfo {
    fn default() -> Self {
        FrameCreateInfo {
            happens_after: Default::default(),
            collect_debug_info: false,
        }
    }
}

/// Determines on which queue a pass will be scheduled.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PassType {
    Graphics,
    Compute,
    Transfer,
    Present {
        swapchain: vk::SwapchainKHR,
        image_index: u32,
    },
}

pub struct Frame<'a> {
    base_serial: u64,
    frame_serial: FrameSerialNumber,
    inner: RefCell<FrameInner<'a>>,
    span: tracing::span::EnteredSpan,
    build_span: tracing::span::EnteredSpan,
}

impl<'a> Frame<'a> {
    /// Returns the context from which this frame was started
    pub fn context(&self) -> RefMut<Context> {
        RefMut::map(self.inner.borrow_mut(), |inner| inner.context)
    }

    /// Returns this frame's serial
    pub fn serial(&self) -> FrameSerialNumber {
        self.frame_serial
    }

    /// Adds a dependency on a GPU future object.
    ///
    /// Execution of the *whole* frame will wait for the operation represented by the future to complete.
    pub fn add_frame_dependency(&self, future: GpuFuture) {
        let mut inner = self.inner.borrow_mut();
        inner.wait_init = inner.wait_init.join(future.serials);
    }

    /// Creates a transient image.
    pub fn create_transient_image(
        &self,
        name: &str,
        memory_info: &ResourceMemoryInfo,
        image_create_info: &ImageResourceCreateInfo,
    ) -> ImageInfo {
        let mut inner = self.inner.borrow_mut();
        inner
            .context
            .create_image(name, memory_info, image_create_info, true)
    }

    /// Creates a transient buffer.
    pub fn create_transient_buffer(
        &self,
        name: &str,
        memory_info: &ResourceMemoryInfo,
        buffer_create_info: &BufferResourceCreateInfo,
    ) -> BufferInfo {
        let mut inner = self.inner.borrow_mut();
        inner
            .context
            .create_buffer(name, memory_info, buffer_create_info, true)
    }

    /// Creates a transient mapped buffer. The caller must ensure that `memory_info` describes
    /// a mappable buffer (HOST_VISIBLE).
    pub fn create_mapped_buffer(
        &self,
        name: &str,
        memory_info: &ResourceMemoryInfo,
        buffer_create_info: &BufferResourceCreateInfo,
    ) -> BufferInfo {
        assert!(memory_info
            .required_flags
            .contains(vk::MemoryPropertyFlags::HOST_VISIBLE));
        let mut inner = self.inner.borrow_mut();
        let buffer_info = inner
            .context
            .create_buffer(name, memory_info, buffer_create_info, true);
        assert!(!buffer_info.mapped_ptr.is_null());
        buffer_info
    }

    pub fn add_graphics_pass(&self, name: &str, handler: impl FnOnce(&mut PassBuilder<'a, '_>)) {
        self.add_pass(name, PassType::Graphics, false, handler)
    }

    /// Starts building a compute pass
    pub fn add_compute_pass(
        &self,
        name: &str,
        async_compute: bool,
        handler: impl FnOnce(&mut PassBuilder),
    ) {
        self.add_pass(name, PassType::Compute, async_compute, handler)
    }

    /// Starts building a transfer pass
    pub fn add_transfer_pass(
        &self,
        name: &str,
        async_transfer: bool,
        handler: impl FnOnce(&mut PassBuilder),
    ) {
        self.add_pass(name, PassType::Transfer, async_transfer, handler)
    }

    /// Presents a swapchain image to the associated swapchain.
    pub fn present(&self, name: &str, image: &SwapchainImage) {
        self.add_pass(
            name,
            PassType::Present {
                swapchain: image.swapchain_handle,
                image_index: image.image_index,
            },
            false,
            |builder| {
                builder.register_image_access_2(
                    image.image_info.id,
                    vk::AccessFlags::MEMORY_READ,
                    vk::PipelineStageFlags::ALL_COMMANDS, // ?
                    vk::ImageLayout::PRESENT_SRC_KHR,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                );
            },
        );
    }

    /// Common code for `build_xxx_pass`
    fn add_pass(
        &self,
        name: &str,
        ty: PassType,
        async_pass: bool,
        handler: impl FnOnce(&mut PassBuilder<'a, '_>),
    ) {
        let mut inner = self.inner.borrow_mut();
        // max 65K passes per frame, because of the implementation of automatic barriers
        // TODO remove this limitation
        let frame_index = inner.passes.len();
        assert!(frame_index <= u16::MAX as usize);
        let serial = inner.context.get_next_serial();

        let q = match ty {
            PassType::Compute if async_pass => inner.context.device.queues_info.indices.compute,
            PassType::Transfer if async_pass => inner.context.device.queues_info.indices.transfer,
            PassType::Present { .. } => inner.context.device.queues_info.indices.present,
            _ => inner.context.device.queues_info.indices.graphics,
        } as usize;

        let snn = SubmissionNumber::new(q, serial);

        let pass = Pass::new(name, frame_index, snn);

        let mut builder = PassBuilder {
            frame: &mut *inner,
            pass,
            _span: trace_span!("add_pass", ?snn).entered(),
        };

        handler(&mut builder);

        let mut pass = builder.pass;

        // pass the swapchain and image index if this is a present pass
        // TODO: use a queue operation callback instead
        if let PassType::Present {
            swapchain,
            image_index,
        } = ty
        {
            pass.commands = Some(PassCommands::Present {
                swapchain,
                image_index,
            })
        }

        inner.push_pass(pass);
    }

    /// Uploads the given object to a transient buffer with the given usage flags and returns the
    /// created buffer resource.
    pub fn upload<T: Copy>(
        &self,
        usage: vk::BufferUsageFlags,
        data: &T,
        name: Option<&str>,
    ) -> TypedBufferInfo<T> {
        let byte_size = mem::size_of::<T>();
        let BufferInfo {
            id,
            handle,
            mapped_ptr,
        } = self.create_upload_buffer(usage, byte_size, name);
        unsafe {
            ptr::write(mapped_ptr as *mut T, *data);
        }
        TypedBufferInfo {
            id,
            handle,
            mapped_ptr: mapped_ptr as *mut T,
        }
    }

    /// Uploads the given slice to a transient buffer with the given usage flags and returns the created buffer resource.
    ///
    /// Example:
    /// ```
    /// let vertices : [[f32;2];3] = [[0.0,0.0],[0.0,1.0],[1.0,1.0]];
    /// batch.upload_slice(graal::vk::BufferUsageFlags::VERTEX_BUFFER, &vertices, "vertices");
    /// ```
    pub fn upload_slice<T: Copy>(
        &self,
        usage: vk::BufferUsageFlags,
        data: &[T],
        name: Option<&str>,
    ) -> TypedBufferInfo<T> {
        let byte_size = mem::size_of_val(data);
        let BufferInfo {
            id,
            handle,
            mapped_ptr,
        } = self.create_upload_buffer(usage, byte_size, name);

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr as *mut T, data.len());
        }

        TypedBufferInfo {
            id,
            handle,
            mapped_ptr: mapped_ptr as *mut T,
        }
    }

    /// Allocates a buffer to hold a value of type `T` and returns a reference to the uninitialized
    /// value inside the buffer.
    ///
    /// Example:
    /// ```
    /// let (buf, val) = batch.alloc_upload::<[f32;4]>(graal::vk::BufferUsageFlags::UNIFORM_BUFFER, None);
    /// unsafe { val.as_mut_ptr().write(Default::default()); }
    /// ```
    pub fn alloc_upload<T: Copy>(
        &self,
        usage: vk::BufferUsageFlags,
        name: Option<&str>,
    ) -> TypedBufferInfo<T> {
        let byte_size = mem::size_of::<T>();
        let BufferInfo {
            id,
            handle,
            mapped_ptr,
        } = self.create_upload_buffer(usage, byte_size, name);

        TypedBufferInfo {
            id,
            handle,
            mapped_ptr: mapped_ptr as *mut T,
        }
    }

    /// Allocates a buffer to hold `size` values of type `T` and returns the slice of the uninitialized
    /// values inside the buffer.
    ///
    /// Example:
    /// ```
    /// let (buf, val) = batch.alloc_upload::<[f32;4]>(graal::vk::BufferUsageFlags::UNIFORM_BUFFER, None);
    /// unsafe { val.as_mut_ptr().write(Default::default()); }
    /// ```
    pub fn alloc_upload_slice<T: Copy>(
        &self,
        usage: vk::BufferUsageFlags,
        size: usize,
        name: Option<&str>,
    ) -> TypedBufferInfo<T> {
        let byte_size = mem::size_of::<T>() * size;
        let BufferInfo {
            id,
            handle,
            mapped_ptr,
        } = self.create_upload_buffer(usage, byte_size, name);
        TypedBufferInfo {
            id,
            handle,
            mapped_ptr: mapped_ptr as *mut T,
        }
    }

    /// Creates a transient buffer mapped in host-coherent memory.
    fn create_upload_buffer(
        &self,
        usage: vk::BufferUsageFlags,
        byte_size: usize,
        name: Option<&str>,
    ) -> BufferInfo {
        self.inner.borrow_mut().context.create_buffer(
            name.unwrap_or("upload buffer"),
            &ResourceMemoryInfo::HOST_VISIBLE_COHERENT,
            &BufferResourceCreateInfo {
                usage,
                byte_size: byte_size as u64,
                map_on_create: true,
            },
            true,
        )
    }

    /// Dumps the frame to a JSON object.
    pub fn dump(&self, file_name_prefix: Option<&str>) {
        use serde_json::json;
        use std::fs::File;

        let inner = self.inner.borrow_mut();

        // passes
        let mut passes_json = Vec::new();
        for (pass_index, p) in inner.passes.iter().enumerate() {
            let image_memory_barriers_json: Vec<_> = p
                .image_memory_barriers
                .iter()
                .map(|imb| {
                    let id = inner.context.image_resource_by_handle(imb.image);
                    let name = &inner.context.resources.get(id).unwrap().name;
                    json!({
                        "type": "image",
                        "srcAccessMask": format!("{:?}", imb.src_access_mask),
                        "dstAccessMask": format!("{:?}", imb.dst_access_mask),
                        "oldLayout": format!("{:?}", imb.old_layout),
                        "newLayout": format!("{:?}", imb.new_layout),
                        "handle": format!("{:#x}", imb.image.as_raw()),
                        "id": format!("{:?}", id.data()),
                        "name": name
                    })
                })
                .collect();

            let buffer_memory_barriers_json: Vec<_> = p
                .buffer_memory_barriers
                .iter()
                .map(|bmb| {
                    let id = inner.context.buffer_resource_by_handle(bmb.buffer);
                    let name = &inner.context.resources.get(id).unwrap().name;
                    json!({
                        "type": "buffer",
                        "srcAccessMask": format!("{:?}", bmb.src_access_mask),
                        "dstAccessMask": format!("{:?}", bmb.dst_access_mask),
                        "handle": format!("{:#x}", bmb.buffer.as_raw()),
                        "id": format!("{:?}", id.data()),
                        "name": name
                    })
                })
                .collect();

            let accesses_json: Vec<_> = p
                .accesses
                .iter()
                .map(|a| {
                    let r = inner.context.resources.get(a.id).unwrap();
                    let name = &r.name;
                    let (ty, handle) = match r.kind {
                        ResourceKind::Buffer(ref buf) => ("buffer", buf.handle.as_raw()),
                        ResourceKind::Image(ref img) => ("image", img.handle.as_raw()),
                    };

                    json!({
                        "id": format!("{:?}", a.id.data()),
                        "name": name,
                        "handle": format!("{:#x}", handle),
                        "type": ty,
                        "accessMask": format!("{:?}", a.access_mask),
                    })
                })
                .collect();

            let mut pass_json = json!({
                "name": p.name,
                "queue": p.snn.queue(),
                "serial": p.snn.serial(),
                "accesses": accesses_json,
                "barriers": {
                    "srcStageMask": format!("{:?}", p.src_stage_mask),
                    "dstStageMask": format!("{:?}", p.dst_stage_mask),
                    "imageMemoryBarriers": image_memory_barriers_json,
                    "bufferMemoryBarriers": buffer_memory_barriers_json,
                },
                "wait": {
                    "serials": p.wait_serials.0,
                    "waitDstStages": format!("{:?}", p.wait_dst_stages),
                },
                "waitExternal": !p.external_semaphore_waits.is_empty(),
            });

            // additional debug information
            if inner.collect_sync_debug_info {
                let sync_debug_info = &inner.sync_debug_info[pass_index];

                let mut resource_tracking_json = Vec::new();
                for (id, tracking) in sync_debug_info.tracking.iter() {
                    let name = &inner.context.resources.get(id).unwrap().name;
                    resource_tracking_json.push(json!({
                        "id": format!("{:?}", id.data()),
                        "name": name,
                        "readers": tracking.readers.0,
                        "writerQueue": tracking.writer.queue(),
                        "writerSerial": tracking.writer.serial(),
                        "layout": format!("{:?}", tracking.layout),
                        "availabilityMask": format!("{:?}", tracking.availability_mask),
                        "visibilityMask": format!("{:?}", tracking.visibility_mask),
                        "stages": format!("{:?}", tracking.stages),
                        "binarySemaphore": tracking.wait_binary_semaphore.as_raw(),
                    }));
                }

                let xq_sync_json: Vec<_> =
                    sync_debug_info.xq_sync_table.iter().map(|v| v.0).collect();

                pass_json.as_object_mut().unwrap().insert(
                    "syncDebugInfo".to_string(),
                    json!({
                        "resourceTrackingInfo": resource_tracking_json,
                        "crossQueueSyncTable": xq_sync_json,
                    }),
                );
            }

            passes_json.push(pass_json);
        }

        let frame_json = json!({
            "frameSerial": self.frame_serial.0,
            "baseSerial": self.base_serial,
            "passes": passes_json,
        });

        let file = File::create(format!(
            "{}-{}.json",
            file_name_prefix.unwrap_or("frame"),
            self.frame_serial.0
        ))
        .expect("could not open file for dumping JSON frame information");
        serde_json::to_writer_pretty(file, &frame_json).unwrap();
    }

    /// Finishes building the frame and submits all the passes to the command queues.
    pub fn finish(self) -> GpuFuture {
        // end build span
        self.build_span.exit();

        let mut inner = self.inner.into_inner();
        let context = inner.context;

        /*println!("Passes:");
        for p in inner.passes.iter() {
            println!("- `{}` ({:?})", p.name, p.snn);
            if p.wait_before {
                println!("    semaphore wait:");
                if p.wait_serials[0] != 0 {
                    println!("        0:{}|{:?}", p.wait_serials[0], p.wait_dst_stages[0]);
                }
                if p.wait_serials[1] != 0 {
                    println!("        1:{}|{:?}", p.wait_serials[1], p.wait_dst_stages[1]);
                }
                if p.wait_serials[2] != 0 {
                    println!("        2:{}|{:?}", p.wait_serials[2], p.wait_dst_stages[2]);
                }
                if p.wait_serials[3] != 0 {
                    println!("        3:{}|{:?}", p.wait_serials[3], p.wait_dst_stages[3]);
                }
            }
            println!(
                "    input execution barrier: {:?}->{:?}",
                p.pre_execution_barrier.src_stage_mask, p.pre_execution_barrier.dst_stage_mask
            );
            println!("    input memory barriers:");
            for imb in p.pre_execution_barrier.image_memory_barriers.iter() {
                let id = context.image_resource_by_handle(imb.image);
                print!("        image handle={:?} ", imb.image);
                if !id.is_null() {
                    print!(
                        "(id={:?}, name={})",
                        id,
                        context.resources.get(id).unwrap().name
                    );
                } else {
                    print!("(unknown resource)");
                }
                println!(
                    " access_mask:{:?}->{:?} layout:{:?}->{:?}",
                    imb.src_access_mask, imb.dst_access_mask, imb.old_layout, imb.new_layout
                );
            }
            for bmb in p.pre_execution_barrier.buffer_memory_barriers.iter() {
                let id = context.buffer_resource_by_handle(bmb.buffer);
                print!("        buffer handle={:?} ", bmb.buffer);
                if !id.is_null() {
                    print!(
                        "(id={:?}, name={})",
                        id,
                        context.resources.get(id).unwrap().name
                    );
                } else {
                    print!("(unknown resource)");
                }
                println!(
                    " access_mask:{:?}->{:?}",
                    bmb.src_access_mask, bmb.dst_access_mask
                );
            }

            //println!("    output stage: {:?}", p.output_stage_mask);
            if p.signal_after {
                println!("    semaphore signal: {:?}", p.snn);
            }
        }

        println!("Final resource states: ");
        for &id in inner.temporaries.iter() {
            let resource = context.resources.get(id).unwrap();
            println!("`{}`", resource.name);
            println!("    stages={:?}", resource.tracking.stages);
            println!("    avail={:?}", resource.tracking.availability_mask);
            println!("    vis={:?}", resource.tracking.visibility_mask);
            println!("    layout={:?}", resource.tracking.layout);

            if resource.tracking.has_readers() {
                println!("    readers: ");
                if resource.tracking.readers[0] != 0 {
                    println!("        0:{}", resource.tracking.readers[0]);
                }
                if resource.tracking.readers[1] != 0 {
                    println!("        1:{}", resource.tracking.readers[1]);
                }
                if resource.tracking.readers[2] != 0 {
                    println!("        2:{}", resource.tracking.readers[2]);
                }
                if resource.tracking.readers[3] != 0 {
                    println!("        3:{}", resource.tracking.readers[3]);
                }
            }
            if resource.tracking.has_writer() {
                println!("    writer: {:?}", resource.tracking.writer);
            }
        }*/

        // First, wait for the frames submitted before the last one to finish, for pacing.
        // This also reclaims the resources referenced by the frames that are not in use anymore.
        context.wait_for_frames_in_flight();

        // Allocate and assign memory for all transient resources of this frame.
        let transient_allocations = allocate_memory_for_transients(context, inner.base_serial, &inner.passes, &inner.temporaries);

        // All resources now have a block of device memory assigned. We're ready to
        // build the command buffers and submit them to the device queues.
        let submission_result = context.submit_frame(&mut inner.passes, inner.wait_init);

        let serials = submission_result.signalled_serials;

        // Add this frame to the list of "frames in flight": frames that might be executing on the device.
        // When this frame is completed, all resources of the frame will be automatically recycled.
        // This includes:
        // - device memory blocks for transient allocations
        // - command buffers (in command pools)
        // - image views
        // - framebuffers
        // - descriptor sets
        context.in_flight.push_back(FrameInFlight {
            signalled_serials: serials,
            transient_allocations,
            command_pools: submission_result.command_pools,
            image_views: submission_result.image_views,
            framebuffers: submission_result.framebuffers,
            descriptor_sets: submission_result.descriptor_sets,
            semaphores: submission_result.semaphores,
        });

        context.submitted_frame_count += 1;
        context.dump_state();

        GpuFuture { serials }
    }
}

impl Context {
    /// Starts a new frame. The execution of the frame can optionally be synchronized
    /// with the given future in `happens_after`.
    ///
    /// However, regardless of this, individual passes in the frame may still synchronize with earlier frames
    /// because of resource dependencies.
    pub fn start_frame(&mut self, create_info: FrameCreateInfo) -> Frame {
        let base_serial = self.next_serial;

        let wait_init = create_info.happens_after.serials;

        // Full CPU-side frame processing
        let span = trace_span!("frame", base_serial).entered();
        // DAG build only
        let build_span = trace_span!("DAG build").entered();

        Frame {
            base_serial,
            frame_serial: FrameSerialNumber(self.submitted_frame_count + 1),
            inner: RefCell::new(FrameInner {
                base_serial,
                context: self,
                wait_init,
                temporaries: vec![],
                temporary_set: TemporarySet::new(),
                passes: vec![],
                xq_sync_table: Default::default(),
                collect_sync_debug_info: create_info.collect_debug_info,
                sync_debug_info: Vec::new(),
            }),
            span,
            build_span,
        }
    }
}