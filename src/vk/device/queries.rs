use super::Device;
use crate::Error;
use crate::QueryKind;
use crate::QueryPool;
use crate::vk::query::Query;
use ash::vk;

impl Device {
    pub fn create_query_pool(
        &self,
        kind: crate::QueryKind,
        count: u32,
    ) -> Result<QueryPool, Error> {
        self.caps.check_query_pool(kind, count)?;
        let none = vk::QueryControlFlags::empty();
        let wanted = kind;
        let (kind, flags) = match kind {
            QueryKind::Timestamp => (vk::QueryType::TIMESTAMP, none),
            QueryKind::Occlusion => (vk::QueryType::OCCLUSION, none),
            QueryKind::Precise => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::PRECISE),
        };
        let info = vk::QueryPoolCreateInfo::default()
            .query_type(kind)
            .query_count(count);
        let pool = unsafe {
            self.raw
                .create_query_pool(&info, None)
                .map_err(Error::from)?
        };
        let id = self.res.queries.insert(Query { pool, flags });
        self.res.table.queries.set(id, (wanted, count));
        Ok(id)
    }

    pub fn destroy_query_pool(&mut self, heap: QueryPool) {
        self.res.table.drop_query(heap);
        let query = self.res.queries.remove(heap);
        unsafe { self.raw.destroy_query_pool(query.pool, None) };
    }
}
