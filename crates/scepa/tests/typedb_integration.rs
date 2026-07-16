use std::{env, future::Future};

use rootcause::prelude::{Report, ResultExt};
use scepa_rs::{pipeline::tei, pipeline::typedb::typeql_queries, typedb::TypeDbConfig};
use typedb_driver::{Addresses, Credentials, DriverOptions, DriverTlsConfig, TypeDBDriver};

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

async fn connect(
    database: &str,
) -> Result<scepa_rs::typedb::TypeDbDriver<scepa_rs::typedb::Connected>, Report> {
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
}

async fn delete_database(database: &str) -> Result<(), Report> {
    let driver = TypeDBDriver::new(
        Addresses::try_from_address_str("127.0.0.1:1729")
            .context("failed to parse TypeDB address")?,
        Credentials::new("admin", "password"),
        DriverOptions::new(DriverTlsConfig::disabled()),
    )
    .await
    .context("failed to connect to TypeDB for test database cleanup")?;
    let result = async {
        driver
            .databases()
            .get(database)
            .await
            .context("failed to open the test database for cleanup")?
            .delete()
            .await
            .context("failed to delete the test database")
    }
    .await;
    let close_result = driver
        .force_close()
        .context("failed to close the TypeDB cleanup driver");
    Ok(result.and(close_result)?)
}

async fn assert_state(
    driver: &scepa_rs::typedb::TypeDbDriver<scepa_rs::typedb::Connected>,
) -> Result<(), Report> {
    let document_count = driver
        .count_matches(&format!(
            "match $a isa research-paper, has entity-id \"{A_KEY}\";"
        ))
        .await
        .context("failed to count Paper A")?;
    if document_count != 1 {
        return Err(rootcause::report!(
            "expected one Paper A, found {document_count}"
        ));
    }

    let titled_document_count = driver
        .count_matches(&format!(
            "match $a isa research-paper, has entity-id \"{A_KEY}\", has title \"Paper A\";"
        ))
        .await
        .context("failed to count titled Paper A")?;
    if titled_document_count != 1 {
        return Err(rootcause::report!(
            "expected one titled Paper A, found {titled_document_count}"
        ));
    }

    let citation_count = driver
        .count_matches("match $c isa citation, links (citing: $b, cited: $a);")
        .await
        .context("failed to count citations")?;
    if citation_count != 1 {
        return Err(rootcause::report!(
            "expected one citation, found {citation_count}"
        ));
    }
    Ok(())
}

async fn with_database<F, Fut>(database: &str, operation: F) -> Result<(), Report>
where
    F: FnOnce(scepa_rs::typedb::TypeDbDriver<scepa_rs::typedb::Connected>) -> Fut,
    Fut: Future<Output = Result<(), Report>>,
{
    let driver = match connect(database).await {
        Ok(driver) => driver,
        Err(error) => {
            // Connecting creates the database before applying the schema, so a
            // schema error can still leave a database behind.
            let _ = delete_database(database).await;
            return Err(error);
        }
    };

    let operation_result = operation(driver.clone()).await;
    let disconnect_result = driver.disconnect();
    let delete_result = delete_database(database).await;

    operation_result?;
    disconnect_result?;
    delete_result
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
        with_database(&database, |driver| async move {
            driver.export_queries(typeql_queries(&first)).await?;
            driver.export_queries(typeql_queries(&second)).await?;
            assert_state(&driver).await?;
            driver.export_queries(typeql_queries(&first)).await?;
            driver.export_queries(typeql_queries(&second)).await?;
            assert_state(&driver).await
        })
        .await
        .unwrap();
    }

    let database = format!("scepa-identity-test-{}-concurrent", std::process::id());
    with_database(&database, |driver| async move {
        let first = driver.clone();
        let second = driver.clone();
        let (first_result, second_result) = tokio::join!(
            first.export_queries(typeql_queries(&paper_b())),
            second.export_queries(typeql_queries(&paper_a())),
        );
        first_result?;
        second_result?;
        assert_state(&driver).await
    })
    .await
    .unwrap();
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
