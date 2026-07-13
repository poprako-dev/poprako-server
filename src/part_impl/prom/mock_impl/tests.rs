use super::*;

// defer_batch_records_payloads(Prom::step)(positive): batch deferral should store every record in transaction state.

#[tokio::test]
async fn defer_batch_records_payloads() {
    let mock = Mock::new();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            let ids = ["prom-1".to_string(), "prom-2".to_string()];

            let payloads = [
                Payload::Image(image::Payload::Delete {
                    object_key: "one.png".to_string(),
                }),
                Payload::Image(image::Payload::Delete {
                    object_key: "two.png".to_string(),
                }),
            ];

            let tasks = [
                Task {
                    id: &ids[0],
                    payload: &payloads[0],
                    delay: None,
                },
                Task {
                    id: &ids[1],
                    payload: &payloads[1],
                    delay: None,
                },
            ];

            prom.step(context, &DeferBatch::new(&tasks)).await?;

            Ok::<(), RegularError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 2);

    assert_eq!(snapshot.prom_records[0].id(), "prom-1");

    assert_eq!(snapshot.prom_records[1].id(), "prom-2");
}
