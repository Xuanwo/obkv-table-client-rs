/*-
 * #%L
 * OBKV Table Client Framework
 * %%
 * Copyright (C) 2021 OceanBase
 * %%
 * OBKV Table Client Framework is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the
 * Mulan PSL v2. You may obtain a copy of Mulan PSL v2 at:
 *          http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
 * KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 * #L%
 */

use std::time::SystemTime;
#[allow(unused_imports)]
#[allow(unused)]
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use obkv::{error::CommonErrCode, ObTableClient, ResultCodes, Value};
use tokio::task;

pub struct BaseTest {
    client: Arc<ObTableClient>,
}

impl BaseTest {
    const ROW_NUM: usize = 400;
    const THREAD_NUM: usize = 5;

    pub fn new(client: ObTableClient) -> BaseTest {
        BaseTest {
            client: Arc::new(client),
        }
    }

    pub async fn test_varchar_concurrent(&self, table_name: &'static str) {
        let mut handles = vec![];
        let start = SystemTime::now();
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..BaseTest::THREAD_NUM {
            let client = self.client.clone();
            let counter = counter.clone();
            handles.push(task::spawn(async move {
                for i in 0..BaseTest::ROW_NUM {
                    let key = format!("foo{i}");
                    let value = format!("bar{i}");
                    let result = client
                        .insert_or_update(
                            table_name,
                            vec![Value::from(key.to_owned())],
                            vec!["c2".to_owned()],
                            vec![Value::from(value.to_owned())],
                        )
                        .await
                        .expect("fail to insert_or update");
                    assert_eq!(1, result);

                    let start_time = SystemTime::now();
                    let mut result = client
                        .get(table_name, vec![Value::from(key)], vec!["c2".to_owned()])
                        .await
                        .expect("fail to get");
                    let end_time = SystemTime::now();
                    if end_time.duration_since(start_time).unwrap().as_millis() > 500 {
                        println!(
                            "get time: {:?}",
                            end_time.duration_since(start_time).unwrap().as_millis()
                        );
                    }
                    assert_eq!(1, result.len());
                    let v = result.remove("c2").unwrap();
                    assert!(v.is_string());
                    assert_eq!(value, v.as_string());

                    counter.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(
            BaseTest::ROW_NUM * BaseTest::THREAD_NUM,
            counter.load(Ordering::SeqCst)
        );
        println!(
            "{} seconds for insert_or_update {} rows.",
            start.elapsed().unwrap().as_secs(),
            BaseTest::ROW_NUM * BaseTest::THREAD_NUM
        );
    }

    pub async fn test_bigint_concurrent(&self, table_name: &'static str) {
        let mut handles = vec![];
        let start = SystemTime::now();
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..BaseTest::THREAD_NUM {
            let client = self.client.clone();
            let counter = counter.clone();
            handles.push(task::spawn(async move {
                for i in 0..BaseTest::ROW_NUM {
                    let key: i64 = i.try_into().unwrap();
                    let value = format!("value{i}");
                    let result = client
                        .insert_or_update(
                            table_name,
                            vec![Value::from(key)],
                            vec!["c2".to_owned()],
                            vec![Value::from(value.to_owned())],
                        )
                        .await
                        .expect("fail to insert_or update");
                    assert_eq!(1, result);

                    let mut result = client
                        .get(table_name, vec![Value::from(key)], vec!["c2".to_owned()])
                        .await
                        .expect("fail to get");
                    assert_eq!(1, result.len());
                    let v = result.remove("c2").unwrap();
                    assert!(v.is_string());
                    assert_eq!(value, v.as_string());

                    counter.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(
            BaseTest::THREAD_NUM * BaseTest::ROW_NUM,
            counter.load(Ordering::SeqCst)
        );
        println!(
            "{} seconds for insert_or_update {} rows.",
            start.elapsed().unwrap().as_secs(),
            BaseTest::THREAD_NUM * BaseTest::ROW_NUM
        );
    }

    pub async fn test_varchar_insert(&self, table_name: &str) {
        let client = &self.client;

        let result = client
            .insert(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("bar")],
            )
            .await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(1, result);

        let result = client
            .insert(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;

        let e = result.unwrap_err();
        assert!(e.is_ob_exception());
        assert_eq!(
            ResultCodes::OB_ERR_PRIMARY_KEY_DUPLICATE,
            e.ob_result_code().unwrap()
        );

        let result = client
            .insert(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("bar")],
            )
            .await;
        let e = result.unwrap_err();
        assert!(e.is_ob_exception());
        assert_eq!(
            ResultCodes::OB_ERR_PRIMARY_KEY_DUPLICATE,
            e.ob_result_code().unwrap()
        );
    }

    async fn assert_varchar_get_result(&self, table_name: &str, row_key: &str, expected: &str) {
        let result = self
            .client
            .get(
                table_name,
                vec![Value::from(row_key)],
                vec!["c2".to_owned()],
            )
            .await;
        assert!(result.is_ok());
        let mut result = result.unwrap();
        assert_eq!(1, result.len());
        let value = result.remove("c2").unwrap();
        assert!(value.is_string());
        assert_eq!(expected, value.as_string());
    }

    pub async fn test_varchar_get(&self, table_name: &str) {
        let result = self
            .client
            .get(table_name, vec![Value::from("bar")], vec!["c2".to_owned()])
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        self.assert_varchar_get_result(table_name, "foo", "bar")
            .await;
    }

    pub async fn test_varchar_update(&self, table_name: &str) {
        let result = self
            .client
            .update(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());

        self.assert_varchar_get_result(table_name, "foo", "baz")
            .await;
    }

    pub async fn test_varchar_insert_or_update(&self, table_name: &str) {
        let result = self
            .client
            .insert_or_update(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("quux")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_varchar_get_result(table_name, "foo", "quux")
            .await;

        let result = self
            .client
            .insert_or_update(
                table_name,
                vec![Value::from("bar")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_varchar_get_result(table_name, "bar", "baz")
            .await;
    }

    pub async fn test_varchar_replace(&self, table_name: &str) {
        let result = self
            .client
            .replace(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("bar")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(2, result.unwrap());
        self.assert_varchar_get_result(table_name, "foo", "bar")
            .await;

        let result = self
            .client
            .replace(
                table_name,
                vec![Value::from("bar")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(2, result.unwrap());
        self.assert_varchar_get_result(table_name, "bar", "baz")
            .await;

        let result = self
            .client
            .replace(
                table_name,
                vec![Value::from("unknown")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());

        self.assert_varchar_get_result(table_name, "unknown", "baz")
            .await;
    }

    pub async fn test_varchar_append(&self, table_name: &str) {
        let result = self
            .client
            .append(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("_append")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_varchar_get_result(table_name, "foo", "bar_append")
            .await;
    }

    pub async fn test_varchar_increment(&self, table_name: &str) {
        let result = self
            .client
            .increment(
                table_name,
                vec![Value::from("foo")],
                vec!["c3".to_owned()],
                vec![Value::from(10i64)],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());

        let result = self
            .client
            .get(table_name, vec![Value::from("foo")], vec!["c3".to_owned()])
            .await;
        assert!(result.is_ok());
        let mut result = result.unwrap();
        assert_eq!(1, result.len());
        let value = result.remove("c3").unwrap();
        assert_eq!(10i64, value.as_i64());

        let result = self
            .client
            .increment(
                table_name,
                vec![Value::from("foo")],
                vec!["c3".to_owned()],
                vec![Value::from(15i64)],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());

        let result = self
            .client
            .get(table_name, vec![Value::from("foo")], vec!["c3".to_owned()])
            .await;
        assert!(result.is_ok());
        let mut result = result.unwrap();
        assert_eq!(1, result.len());
        let value = result.remove("c3").unwrap();
        assert_eq!(25i64, value.as_i64());
    }

    pub async fn clean_varchar_table(&self, table_name: &str) {
        let result = self
            .client
            .delete(table_name, vec![Value::from("unknown")])
            .await;
        assert!(result.is_ok());
        let result = self
            .client
            .delete(table_name, vec![Value::from("foo")])
            .await;
        assert!(result.is_ok());
        let result = self
            .client
            .delete(table_name, vec![Value::from("bar")])
            .await;
        assert!(result.is_ok());
        let result = self
            .client
            .delete(table_name, vec![Value::from("baz")])
            .await;
        assert!(result.is_ok());

        for i in 0..100 {
            let key = format!("foo{i}");
            let result = self.client.delete(table_name, vec![Value::from(key)]).await;
            assert!(result.is_ok());
        }
    }

    pub async fn clean_bigint_table(&self, table_name: &str) {
        for i in 0..100 {
            let key: i64 = i;
            let result = self.client.delete(table_name, vec![Value::from(key)]).await;
            assert!(result.is_ok());
        }
    }

    pub async fn test_blob_insert(&self, table_name: &str) {
        let client = &self.client;
        let bs = "hello".as_bytes();

        let result = client
            .insert(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from(bs)],
            )
            .await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(1, result);

        let result = client
            .insert(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from(bs)],
            )
            .await;

        let e = result.unwrap_err();
        assert!(e.is_ob_exception());
        assert_eq!(
            ResultCodes::OB_ERR_PRIMARY_KEY_DUPLICATE,
            e.ob_result_code().unwrap()
        );

        //test insert string
        let result = client
            .insert(
                table_name,
                vec![Value::from("qux")],
                vec!["c2".to_owned()],
                vec![Value::from("qux")],
            )
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(1, result);
    }

    async fn assert_blob_get_result(&self, table_name: &str, row_key: &str, expected: &str) {
        let result = self
            .client
            .get(
                table_name,
                vec![Value::from(row_key)],
                vec!["c2".to_owned()],
            )
            .await;
        assert!(result.is_ok());
        let mut result = result.unwrap();
        assert_eq!(1, result.len());
        let value = result.remove("c2").unwrap();
        assert!(value.is_bytes());
        assert_eq!(expected, String::from_utf8(value.as_bytes()).unwrap());
    }

    pub async fn test_blob_get(&self, table_name: &str) {
        let result = self
            .client
            .get(table_name, vec![Value::from("bar")], vec!["c2".to_owned()])
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        self.assert_blob_get_result(table_name, "foo", "hello")
            .await;
        self.assert_blob_get_result(table_name, "qux", "qux").await;
    }

    pub async fn test_blob_update(&self, table_name: &str) {
        let result = self
            .client
            .update(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("baz".as_bytes())],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_blob_get_result(table_name, "foo", "baz").await;

        let result = self
            .client
            .update(
                table_name,
                vec![Value::from("qux")],
                vec!["c2".to_owned()],
                vec![Value::from("baz".as_bytes())],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_blob_get_result(table_name, "qux", "baz").await;
    }

    pub async fn test_blob_insert_or_update(&self, table_name: &str) {
        let result = self
            .client
            .insert_or_update(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("quux".as_bytes())],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_blob_get_result(table_name, "foo", "quux").await;

        let result = self
            .client
            .insert_or_update(
                table_name,
                vec![Value::from("bar")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
        self.assert_blob_get_result(table_name, "bar", "baz").await;
    }

    pub async fn test_blob_replace(&self, table_name: &str) {
        let result = self
            .client
            .replace(
                table_name,
                vec![Value::from("foo")],
                vec!["c2".to_owned()],
                vec![Value::from("bar")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(2, result.unwrap());
        self.assert_blob_get_result(table_name, "foo", "bar").await;

        let result = self
            .client
            .replace(
                table_name,
                vec![Value::from("bar")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(2, result.unwrap());
        self.assert_blob_get_result(table_name, "bar", "baz").await;

        let result = self
            .client
            .replace(
                table_name,
                vec![Value::from("baz")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());

        self.assert_blob_get_result(table_name, "baz", "baz").await;
    }

    pub async fn clean_blob_table(&self, table_name: &str) {
        self.client
            .delete(table_name, vec![Value::from("qux")])
            .await
            .expect("fail to delete row");
        self.client
            .delete(table_name, vec![Value::from("bar")])
            .await
            .expect("fail to delete row");
        self.client
            .delete(table_name, vec![Value::from("baz")])
            .await
            .expect("fail to delete row");
        self.client
            .delete(table_name, vec![Value::from("foo")])
            .await
            .expect("fail to delete row");
    }

    pub async fn test_varchar_exceptions(&self, table_name: &str) {
        // delete exception_key
        let result = self
            .client
            .delete(table_name, vec![Value::from("exception_key")])
            .await;
        // assert result is ok
        assert!(result.is_ok());

        //table not exists
        let result = self
            .client
            .insert(
                "not_exist_table",
                vec![Value::from("exception_key")],
                vec!["c2".to_owned()],
                vec![Value::from("baz")],
            )
            .await;

        let e = result.unwrap_err();
        assert!(e.is_ob_exception());
        assert_eq!(
            ResultCodes::OB_ERR_UNKNOWN_TABLE,
            e.ob_result_code().unwrap()
        );

        // column not found
        let result = self
            .client
            .insert(
                table_name,
                vec![Value::from("exception_key")],
                vec!["c4".to_owned()],
                vec![Value::from("baz")],
            )
            .await;

        let e = result.unwrap_err();
        assert!(e.is_ob_exception());
        if self.client.ob_vsn_major() >= 4 {
            assert_eq!(
                ResultCodes::OB_ERR_BAD_FIELD_ERROR,
                e.ob_result_code().unwrap()
            );
            assert!(e
                .ob_result_msg()
                .unwrap()
                .contains("message:Unknown column 'c4' in"));
        } else {
            assert_eq!(
                ResultCodes::OB_ERR_COLUMN_NOT_FOUND,
                e.ob_result_code().unwrap()
            );
        }

        // TODO
        // column/rowkey type error
        // let result = self.client.insert(
        //     table_name,
        //     vec![Value::from(1)],
        //     vec!["c2".to_owned()],
        //     vec![Value::from("baz")],
        // );
        // let e = result.unwrap_err();
        // assert!(e.is_ob_exception());
        // assert_eq!(ResultCodes::OB_OBJ_TYPE_ERROR, e.ob_result_code().unwrap());

        let result = self
            .client
            .insert(
                table_name,
                vec![Value::from("exception_key")],
                vec!["c2".to_owned()],
                vec![Value::from(1)],
            )
            .await;
        let e = result.unwrap_err();
        assert!(e.is_ob_exception());
        if self.client.ob_vsn_major() >= 4 {
            assert_eq!(
                ResultCodes::OB_KV_COLUMN_TYPE_NOT_MATCH,
                e.ob_result_code().unwrap()
            );
            assert!(e
                .ob_result_msg()
                .unwrap()
                .contains("Column type for 'c2' not match, schema column type is 'VARCHAR'"));
        } else {
            assert_eq!(ResultCodes::OB_OBJ_TYPE_ERROR, e.ob_result_code().unwrap());
        }

        // null value
        let result = self
            .client
            .insert(
                table_name,
                vec![Value::from("exception_key")],
                vec!["c2".to_owned()],
                vec![Value::default()],
            )
            .await;
        // assert result is ok
        assert!(result.is_ok());

        // delete exception_key
        let result = self
            .client
            .delete(table_name, vec![Value::from("exception_key")])
            .await;
        // assert result is ok
        assert!(result.is_ok());
    }

    pub async fn insert_query_test_record(&self, table_name: &str, row_key: &str, value: &str) {
        let result = self
            .client
            .insert_or_update(
                table_name,
                vec![Value::from(row_key)],
                vec!["c2".to_owned()],
                vec![Value::from(value)],
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(1, result.unwrap());
    }

    pub async fn test_stream_query(&self, table_name: &str) {
        println!("test_stream_query for table name: {table_name} is unsupported now");
        // for i in 0..10 {
        //     let key = format!("{}", i);
        //     self.insert_query_test_record(table_name, &key, &key);
        // }
        //
        // let query = self
        //     .client
        //     .query(table_name)
        //     .batch_size(2)
        //     .select(vec!["c2".to_owned()])
        //     .primary_index()
        //     .add_scan_range(vec![Value::from("0")], true,
        // vec![Value::from("9")], true);
        //
        // let result_set = query.execute();
        //
        // assert!(result_set.is_ok());
        //
        // let result_set = result_set.unwrap();
        //
        // assert_eq!(0, result_set.cache_size());
        //
        // let mut i = 0;
        // for row in result_set {
        //     assert!(row.is_ok());
        //     let mut row = row.unwrap();
        //     let key = format!("{}", i);
        //     assert_eq!(key, row.remove("c2").unwrap().as_string());
        //     i = i + 1;
        // }
        //
        // assert_eq!(10, i);
    }

    pub async fn test_query(&self, table_name: &str) {
        self.insert_query_test_record(table_name, "123", "123c2")
            .await;
        self.insert_query_test_record(table_name, "124", "124c2")
            .await;
        self.insert_query_test_record(table_name, "234", "234c2")
            .await;
        self.insert_query_test_record(table_name, "456", "456c2")
            .await;
        self.insert_query_test_record(table_name, "567", "567c2")
            .await;

        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("123")],
                true,
                vec![Value::from("567")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(5, result_set.cache_size());

        for i in 0..5 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("234c2", row.remove("c2").unwrap().as_string()),
                3 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                4 => assert_eq!("567c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        //reverse order
        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .scan_order(false)
            .add_scan_range(
                vec![Value::from("123")],
                true,
                vec![Value::from("567")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(5, result_set.cache_size());

        for i in 0..5 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("567c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("234c2", row.remove("c2").unwrap().as_string()),
                3 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                4 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // >= 123 && <= 123
        let mut query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("123")],
                true,
                vec![Value::from("123")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(1, result_set.cache_size());

        assert_eq!(
            "123c2",
            result_set
                .next()
                .await
                .unwrap()
                .unwrap()
                .remove("c2")
                .unwrap()
                .as_string()
        );

        // >= 124 && <= 456
        query.clear();
        let mut query = query
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("124")],
                true,
                vec![Value::from("456")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(3, result_set.cache_size());

        for i in 0..3 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("234c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // > 123 && < 567
        query.clear();
        let mut query = query
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("123")],
                false,
                vec![Value::from("567")],
                false,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(3, result_set.cache_size());

        for i in 0..3 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("234c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // > 123 && <= 567
        query.clear();
        let mut query = query
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("123")],
                false,
                vec![Value::from("567")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(4, result_set.cache_size());

        for i in 0..4 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("234c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                3 => assert_eq!("567c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // >=123 && < 567
        query.clear();
        let mut query = query
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("123")],
                true,
                vec![Value::from("567")],
                false,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(4, result_set.cache_size());

        for i in 0..4 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("234c2", row.remove("c2").unwrap().as_string()),
                3 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // >= 12 && <= 126
        query.clear();
        let mut query = query
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("12")],
                true,
                vec![Value::from("126")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(2, result_set.cache_size());

        for i in 0..2 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // (>=12 && <=126) || (>="456" && <="567")
        query.clear();
        let query = query
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("12")],
                true,
                vec![Value::from("126")],
                true,
            )
            .add_scan_range(
                vec![Value::from("456")],
                true,
                vec![Value::from("567")],
                true,
            );

        let result_set = query.execute().await;
        assert!(result_set.is_ok());
        let mut result_set = result_set.unwrap();
        assert_eq!(4, result_set.cache_size());

        for i in 0..4 {
            let result = result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                3 => assert_eq!("567c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        // (>=124 && <=124)
        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("124")],
                true,
                vec![Value::from("124")],
                true,
            );

        let result_set = query.execute().await;

        assert!(result_set.is_ok());

        let mut result_set = result_set.unwrap();

        assert_eq!(1, result_set.cache_size());

        assert_eq!(
            "124c2",
            result_set
                .next()
                .await
                .unwrap()
                .unwrap()
                .remove("c2")
                .unwrap()
                .as_string()
        );

        // (>=124 && <=123)
        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .add_scan_range(
                vec![Value::from("124")],
                true,
                vec![Value::from("123")],
                true,
            );

        let result_set = query.execute().await;
        assert!(result_set.is_ok());
        let result_set = result_set.unwrap();
        assert_eq!(0, result_set.cache_size());

        // TODO batch not supported in query now
        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            // .batch_size(1)
            .add_scan_range(
                vec![Value::from("12")],
                true,
                vec![Value::from("126")],
                true,
            )
            .add_scan_range(
                vec![Value::from("456")],
                true,
                vec![Value::from("567")],
                true,
            );

        let query_result_set = query.execute().await;
        assert!(query_result_set.is_ok());
        let mut query_result_set = query_result_set.unwrap();
        assert_eq!(4, query_result_set.cache_size());

        for i in 0..4 {
            let result = query_result_set.next().await.unwrap();
            assert!(result.is_ok());
            let mut row = result.unwrap();
            match i {
                0 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                1 => assert_eq!("124c2", row.remove("c2").unwrap().as_string()),
                2 => assert_eq!("456c2", row.remove("c2").unwrap().as_string()),
                3 => assert_eq!("567c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }

        //Close result set before usage
        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .batch_size(1)
            .add_scan_range(
                vec![Value::from("12")],
                true,
                vec![Value::from("126")],
                true,
            )
            .add_scan_range(
                vec![Value::from("456")],
                true,
                vec![Value::from("567")],
                true,
            );

        let result_set = query.execute().await;
        assert!(result_set.is_ok());
        let mut result_set = result_set.unwrap();
        assert_eq!(0, result_set.cache_size());
        for i in 0..1 {
            let row = result_set.next().await.unwrap();
            assert!(row.is_ok());
            let mut row = row.unwrap();
            match i {
                0 => assert_eq!("123c2", row.remove("c2").unwrap().as_string()),
                _ => unreachable!(),
            }
        }
        let ret = result_set.close().await;
        assert!(ret.is_ok());

        match result_set.next().await {
            Some(Err(e)) => {
                assert!(e.is_common_err());
                assert_eq!(CommonErrCode::AlreadyClosed, e.common_err_code().unwrap());
            }
            _other => unreachable!(),
        }

        // TODO
        // Session timeout expired
        // let query = self
        //     .client
        //     .query(table_name)
        //     .select(vec!["c2".to_owned()])
        //     .primary_index()
        //     .operation_timeout(Duration::from_secs(1))
        //     .batch_size(1)
        //     .add_scan_range(
        //         vec![Value::from("12")],
        //         true,
        //         vec![Value::from("126")],
        //         true,
        //     )
        //     .add_scan_range(
        //         vec![Value::from("456")],
        //         true,
        //         vec![Value::from("567")],
        //         true,
        //     );
        //
        // let result_set = query.execute();
        // assert!(result_set.is_ok());
        // let mut result_set = result_set.unwrap();
        // assert_eq!(0, result_set.cache_size());
        //
        // let row = result_set.next();
        // assert!(row.is_some());
        //
        // thread::sleep(Duration::from_secs(2));
        //
        // let e = result_set.next().unwrap().unwrap_err();
        // assert!(e.is_ob_exception());
        // // the exception is OB_TIMEOUT on ob2.x and is OB_TRANS_ROLLBACKED in ob1.x.
        // let code = e.ob_result_code().unwrap();
        // assert!(code == ResultCodes::OB_TRANS_ROLLBACKED || code ==
        // ResultCodes::OB_TRANS_TIMEOUT);

        // TODO
        //In session timeout
        let query = self
            .client
            .query(table_name)
            .select(vec!["c2".to_owned()])
            .primary_index()
            .operation_timeout(Duration::from_secs(3))
            .batch_size(1)
            .add_scan_range(
                vec![Value::from("12")],
                true,
                vec![Value::from("126")],
                true,
            )
            .add_scan_range(
                vec![Value::from("456")],
                true,
                vec![Value::from("567")],
                true,
            );

        let result_set = query.execute().await;
        assert!(result_set.is_ok());
        let mut result_set = result_set.unwrap();
        assert_eq!(0, result_set.cache_size());

        let row = result_set.next().await;
        assert!(row.is_some());

        tokio::time::sleep(Duration::from_secs(2)).await;
        let row = result_set.next().await;
        assert!(row.is_some());
        let row = row.unwrap();
        // println!(
        //     "TODO: could not find data, row error code: {:?}",
        //     row.unwrap_err().ob_result_code()
        // );
        assert!(row.is_ok());
    }
}
