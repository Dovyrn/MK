use super::Device;
use crate::QueryPool;
use crate::query_kind;
use crate::vk::query::Query;
use ash::vk;

impl Device {
    pub fn create_query_pool(&mut self, kind: u32, count: u32) -> Result<QueryPool, String> {
        self.caps.check_query_pool(kind, count)?;
        let none = vk::QueryControlFlags::empty();
        let wanted = kind;
        let (kind, flags) = match kind {
            query_kind::TIMESTAMP => (vk::QueryType::TIMESTAMP, none),
            query_kind::OCCLUSION => (vk::QueryType::OCCLUSION, none),
            query_kind::PRECISE => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::PRECISE),
            _ => unreachable!(),
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

    pub fn destroy_query_pool(&mut self, heap: QueryPool) {
        self.res.table.queries.clear(heap);
        let query = self.res.queries.remove(heap);
        unsafe { self.raw.destroy_query_pool(query.pool, None) };
    }
}
