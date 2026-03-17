use duckdb::Connection;

#[test]
fn test_get_envelopes_with_params_iter() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE envelopes (id VARCHAR, account_id UBIGINT)", []).unwrap();
    
    conn.execute("INSERT INTO envelopes VALUES ('mail_1', 1), ('mail_2', 1), ('mail_3', 2)", []).unwrap();

    let search_ids = vec!["mail_1", "mail_2"];
    let acc_id = 1u64;

    let placeholders = search_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!("SELECT id FROM envelopes WHERE account_id = ? AND id IN ({})", placeholders);
    
    let mut params: Vec<&dyn duckdb::ToSql> = Vec::new();
    params.push(&acc_id);
    for id in &search_ids {
        params.push(id);
    }

    let mut stmt = conn.prepare(&query).unwrap();
    let res: Vec<String> = stmt.query_map(duckdb::params_from_iter(params), |r| r.get(0)).unwrap()
        .map(|r| r.unwrap()).collect();
    println!("{:#?}", &res);
    assert_eq!(res.len(), 2);
    assert!(res.contains(&"mail_1".to_string()));
}