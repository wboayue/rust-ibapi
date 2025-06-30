#[cfg(test)]
mod tests {
    use crate::accounts::AccountSummaryTags;
    use crate::stubs::MessageBusStub;
    use crate::Client;
    use std::sync::Arc;

    #[test]
    fn test_only_one_account_summary_allowed() {
        let message_bus = Arc::new(MessageBusStub::default());
        let client = Client::stubbed(message_bus, 176);

        // Create first subscription - should succeed
        let subscription1 = client
            .account_summary("All", &[AccountSummaryTags::NET_LIQUIDATION])
            .expect("First account_summary should succeed");

        // Try to create second subscription - should fail
        let result = client.account_summary("All", &[AccountSummaryTags::TOTAL_CASH_VALUE]);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Only one account_summary subscription is allowed"));

        // Cancel first subscription
        subscription1.cancel();

        // Now second subscription should succeed
        let subscription2 = client
            .account_summary("All", &[AccountSummaryTags::TOTAL_CASH_VALUE])
            .expect("Second account_summary should succeed after cancelling first");

        // Clean up
        subscription2.cancel();
    }
}
