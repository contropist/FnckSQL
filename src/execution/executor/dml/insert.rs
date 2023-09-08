use std::collections::HashMap;
use std::sync::Arc;
use futures_async_stream::try_stream;
use crate::catalog::TableName;
use crate::execution::executor::BoxedExecutor;
use crate::execution::ExecutorError;
use crate::storage::{Storage, Table};
use crate::types::{ColumnId, IdGenerator};
use crate::types::tuple::Tuple;
use crate::types::value::{DataValue, ValueRef};

pub struct Insert { }

impl Insert {
    #[try_stream(boxed, ok = Tuple, error = ExecutorError)]
    pub async fn execute(table_name: TableName, input: BoxedExecutor, storage: impl Storage) {
        if let (Some(table_catalog), Some(mut table)) =
            (storage.table_catalog(&table_name).await, storage.table(&table_name).await)
        {
            #[for_await]
            for tuple in input {
                let Tuple { columns, values, .. } = tuple?;
                let mut tuple_map: HashMap<ColumnId, ValueRef> = values
                    .into_iter()
                    .enumerate()
                    .map(|(i, value)| (columns[i].id, value))
                    .collect();

                let all_columns = table_catalog.all_columns_with_id();

                let mut tuple = Tuple {
                    id: Some(IdGenerator::build() as usize),
                    columns: Vec::with_capacity(all_columns.len()),
                    values: Vec::with_capacity(all_columns.len()),
                };

                for (col_id, col) in all_columns {
                    let value = tuple_map.remove(col_id)
                        .unwrap_or_else(|| Arc::new(DataValue::none(col.datatype())));

                    tuple.columns.push(col.clone());
                    tuple.values.push(value)
                }

                table.append(tuple)?;
            }
            table.commit().await?;
        }
    }
}