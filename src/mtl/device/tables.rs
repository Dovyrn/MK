use super::{Device, id};
use crate::mtl::table::Table;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLComputePipelineState, MTLIntersectionFunctionTable, MTLIntersectionFunctionTableDescriptor,
    MTLVisibleFunctionTable, MTLVisibleFunctionTableDescriptor,
};

impl Device {
    pub fn create_function_table(
        &mut self,
        pipeline: u32,
        functions: &[&str],
        intersection: bool,
    ) -> Result<u32, String> {
        let state = &self.res.pipelines.get(pipeline).state;
        let handles = functions
            .iter()
            .map(|f| {
                state
                    .functionHandleWithName(&NSString::from_str(f))
                    .ok_or(format!("{f} is not linked into the pipeline"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let table = if intersection {
            let desc = MTLIntersectionFunctionTableDescriptor::new();
            desc.setFunctionCount(handles.len());
            let table = state
                .newIntersectionFunctionTableWithDescriptor(&desc)
                .ok_or("intersection function table")?;
            for (i, h) in handles.iter().enumerate() {
                table.setFunction_atIndex(Some(h), i);
            }
            self.residency.add(ProtocolObject::from_ref(&*table));
            Table::Intersection(table)
        } else {
            let desc = MTLVisibleFunctionTableDescriptor::new();
            unsafe { desc.setFunctionCount(handles.len()) };
            let table = state
                .newVisibleFunctionTableWithDescriptor(&desc)
                .ok_or("visible function table")?;
            for (i, h) in handles.iter().enumerate() {
                unsafe { table.setFunction_atIndex(Some(h), i) };
            }
            self.residency.add(ProtocolObject::from_ref(&*table));
            Table::Visible(table)
        };
        Ok(self.res.tables.insert(table))
    }

    pub fn destroy_function_table(&mut self, table: u32) {
        let table = self.res.tables.remove(table);
        self.defer(move |r| match table {
            Table::Visible(t) => r.remove(ProtocolObject::from_ref(&*t)),
            Table::Intersection(t) => r.remove(ProtocolObject::from_ref(&*t)),
        });
    }

    pub fn function_table_handle(&self, table: u32) -> u64 {
        match self.res.tables.get(table) {
            Table::Visible(t) => id(t.gpuResourceID()),
            Table::Intersection(t) => id(t.gpuResourceID()),
        }
    }
}
