use super::Device;
use crate::query;
use crate::vk::query::Query;
use ash::vk;

impl Device {
    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let caps = self.caps;
        let none = vk::QueryControlFlags::empty();
        let wanted = kind;
        let (kind, flags) = match kind {
            query::TIMESTAMP if !caps.timestamps => {
                return Err("timestamps are not supported".into());
            }
            query::TIMESTAMP => (vk::QueryType::TIMESTAMP, none),
            query::STATISTICS if !caps.statistics => {
                return Err("pipeline statistics are not supported".into());
            }
            query::STATISTICS => (vk::QueryType::PIPELINE_STATISTICS, none),
            query::COMPACTED if !caps.ray => {
                return Err("VK_KHR_acceleration_structure is not enabled".into());
            }
            query::COMPACTED => (
                vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR,
                none,
            ),
            query::OCCLUSION => (vk::QueryType::OCCLUSION, none),
            query::PRECISE if !caps.precise => {
                return Err("precise occlusion queries are not supported".into());
            }
            query::PRECISE => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::PRECISE),
            k => panic!("unknown query kind {k}"),
        };
        let statistics = if kind == vk::QueryType::PIPELINE_STATISTICS {
            vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_VERTICES
                | vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_PRIMITIVES
                | vk::QueryPipelineStatisticFlags::VERTEX_SHADER_INVOCATIONS
                | vk::QueryPipelineStatisticFlags::CLIPPING_INVOCATIONS
                | vk::QueryPipelineStatisticFlags::CLIPPING_PRIMITIVES
                | vk::QueryPipelineStatisticFlags::FRAGMENT_SHADER_INVOCATIONS
                | vk::QueryPipelineStatisticFlags::COMPUTE_SHADER_INVOCATIONS
        } else {
            vk::QueryPipelineStatisticFlags::empty()
        };
        let info = vk::QueryPoolCreateInfo::default()
            .query_type(kind)
            .query_count(count)
            .pipeline_statistics(statistics);
        let pool = unsafe {
            self.raw
                .create_query_pool(&info, None)
                .map_err(|e| e.to_string())?
        };
        let stride = statistics.as_raw().count_ones().max(1) as u64 * 8;
        let id = self.res.queries.insert(Query {
            pool,
            flags,
            stride,
        });
        self.res.table.heaps.set(id, (wanted, count));
        Ok(id)
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.res.table.heaps.clear(heap);
        self.defer(move |d| {
            let query = d.res.queries.remove(heap);
            unsafe { d.raw.destroy_query_pool(query.pool, None) };
        });
    }
}
