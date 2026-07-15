use std::env;

use scepa_rs::{pipeline::tei, pipeline::typedb::typeql_queries, typedb::TypeDbConfig};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const A_KEY: &str = "document:doi:10.1234/a";

fn enabled() -> bool {
    env::var("TYPEDB_INTEGRATION").as_deref() == Ok("1")
}

fn paper_a() -> scepa_rs::domain::DocumentWithChunks {
    tei::parse_with_pdf_hash(
        r#"
        <TEI><teiHeader><fileDesc>
          <titleStmt><title type="main">Paper A</title></titleStmt>
          <sourceDesc><biblStruct><analytic/>
            <idno type="DOI">10.1234/a</idno>
          </biblStruct></sourceDesc>
        </fileDesc></teiHeader><text><body><p>Rich metadata.</p></body></text></TEI>
        "#,
        HASH_A,
    )
    .unwrap()
}

fn paper_b() -> scepa_rs::domain::DocumentWithChunks {
    tei::parse_with_pdf_hash(
        r#"
        <TEI><teiHeader><fileDesc>
          <titleStmt><title type="main">Paper B</title></titleStmt>
          <sourceDesc><biblStruct><analytic/>
            <idno type="DOI">10.1234/b</idno>
          </biblStruct></sourceDesc>
        </fileDesc></teiHeader><text><back><listBibl>
          <biblStruct xml:id="ref-a"><analytic><title>Paper A</title></analytic>
            <idno type="DOI">https://doi.org/10.1234/a</idno>
          </biblStruct>
        </listBibl></back></text></TEI>
        "#,
        HASH_B,
    )
    .unwrap()
}

async fn connect(database: &str) -> scepa_rs::typedb::TypeDbDriver<scepa_rs::typedb::Connected> {
    scepa_rs::typedb::TypeDbDriver::default()
        .connect(&TypeDbConfig::new(
            "127.0.0.1:1729",
            database,
            "admin",
            "password",
            false,
            true,
        ))
        .await
        .unwrap()
}

async fn assert_state(driver: &scepa_rs::typedb::TypeDbDriver<scepa_rs::typedb::Connected>) {
    assert_eq!(
        driver
            .count_matches(&format!(
                "match $a isa research-paper, has entity-id \"{A_KEY}\";"
            ))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        driver
            .count_matches(&format!(
                "match $a isa research-paper, has entity-id \"{A_KEY}\", has title \"Paper A\";"
            ))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        driver
            .count_matches("match $c isa citation, links (citing: $b, cited: $a);")
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn ingestion_orders_and_concurrency_converge() {
    if !enabled() {
        return;
    }

    for (suffix, first, second) in [
        ("b-then-a", paper_b(), paper_a()),
        ("a-then-b", paper_a(), paper_b()),
    ] {
        let database = format!("scepa-identity-test-{}-{suffix}", std::process::id());
        let driver = connect(&database).await;
        driver.export_queries(typeql_queries(&first)).await.unwrap();
        driver
            .export_queries(typeql_queries(&second))
            .await
            .unwrap();
        assert_state(&driver).await;
        driver.export_queries(typeql_queries(&first)).await.unwrap();
        driver
            .export_queries(typeql_queries(&second))
            .await
            .unwrap();
        assert_state(&driver).await;
        driver.disconnect().unwrap();
    }

    let database = format!("scepa-identity-test-{}-concurrent", std::process::id());
    let driver = connect(&database).await;
    let first = driver.clone();
    let second = driver.clone();
    let (first_result, second_result) = tokio::join!(
        first.export_queries(typeql_queries(&paper_b())),
        second.export_queries(typeql_queries(&paper_a())),
    );
    first_result.unwrap();
    second_result.unwrap();
    assert_state(&driver).await;
    driver.disconnect().unwrap();
}

#[tokio::test]
async fn inspect_development_database() {
    if env::var("TYPEDB_INSPECT_DEV").as_deref() != Ok("1") {
        return;
    }
    let driver = scepa_rs::typedb::TypeDbDriver::default()
        .connect(&TypeDbConfig::new(
            "127.0.0.1:1729",
            "scepa",
            "admin",
            "password",
            false,
            false,
        ))
        .await
        .unwrap();
    for (label, query) in [
        ("documents", "match $e isa document;"),
        ("people", "match $e isa person;"),
        ("citations", "match $r isa citation;"),
        (
            "unkeyed-documents",
            "match $e isa document; not { $e has entity-id $_; };",
        ),
        (
            "unkeyed-people",
            "match $e isa person; not { $e has entity-id $_; };",
        ),
    ] {
        println!("{label}={}", driver.count_matches(query).await.unwrap());
    }
    driver.disconnect().unwrap();
}
