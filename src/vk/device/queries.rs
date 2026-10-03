use super::Device;
use crate::query;
use crate::vk::query::Query;
use ash::vk;

impl Device {
    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        self.caps.queries(kind, count)?;
        let caps = self.caps;
        let none = vk::QueryControlFlags::empty();
        let wanted = kind;
        let (kind, flags) = match kind {
            query::TIMESTAMP if !caps.timestamps => {
                return Err("timestamps are not supported".into());
            }
            query::TIMESTAMP => (vk::QueryType::TIMESTAMP, none),
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
        let info = vk::QueryPoolCreateInfo::default()
            .query_type(kind)
            .query_count(count);
        let pool = unsafe {
            self.raw
                .create_query_pool(&info, None)
                .map_err(|e| e.to_string())?
        };
        let id = self.res.queries.insert(Query { pool, flags });
        self.res.table.queries.set(id, (wanted, count));
        Ok(id)
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.res.table.queries.clear(heap);
        self.defer(move |d| {
            let query = d.res.queries.remove(heap);
            unsafe { d.raw.destroy_query_pool(query.pool, None) };
        });
    }
}
