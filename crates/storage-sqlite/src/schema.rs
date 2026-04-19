// @generated automatically by Diesel CLI.

diesel::table! {
    account_tax_profiles (account_id) {
        account_id -> Text,
        jurisdiction -> Text,
        regime -> Text,
        opened_on -> Nullable<Text>,
        closed_on -> Nullable<Text>,
        metadata -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    accounts (id) {
        id -> Text,
        name -> Text,
        account_type -> Text,
        group -> Nullable<Text>,
        currency -> Text,
        is_default -> Bool,
        is_active -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        platform_id -> Nullable<Text>,
        account_number -> Nullable<Text>,
        meta -> Nullable<Text>,
        provider -> Nullable<Text>,
        provider_account_id -> Nullable<Text>,
        is_archived -> Bool,
        tracking_mode -> Text,
    }
}

diesel::table! {
    activities (id) {
        id -> Text,
        account_id -> Text,
        asset_id -> Nullable<Text>,
        activity_type -> Text,
        activity_type_override -> Nullable<Text>,
        source_type -> Nullable<Text>,
        subtype -> Nullable<Text>,
        status -> Text,
        activity_date -> Text,
        settlement_date -> Nullable<Text>,
        quantity -> Nullable<Text>,
        unit_price -> Nullable<Text>,
        amount -> Nullable<Text>,
        fee -> Nullable<Text>,
        currency -> Text,
        fx_rate -> Nullable<Text>,
        notes -> Nullable<Text>,
        metadata -> Nullable<Text>,
        source_system -> Nullable<Text>,
        source_record_id -> Nullable<Text>,
        source_group_id -> Nullable<Text>,
        idempotency_key -> Nullable<Text>,
        import_run_id -> Nullable<Text>,
        is_user_modified -> Integer,
        needs_review -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    import_account_templates (id) {
        id -> Text,
        account_id -> Text,
        context_kind -> Text,
        source_system -> Text,
        template_id -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    import_templates (id) {
        id -> Text,
        name -> Text,
        scope -> Text,
        kind -> Text,
        source_system -> Text,
        config_version -> Integer,
        config -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ai_messages (id) {
        id -> Text,
        thread_id -> Text,
        role -> Text,
        content_json -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    ai_thread_tags (id) {
        id -> Text,
        thread_id -> Text,
        tag -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    ai_threads (id) {
        id -> Text,
        title -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        config_snapshot -> Nullable<Text>,
        is_pinned -> Integer,
    }
}

diesel::table! {
    app_settings (setting_key) {
        setting_key -> Text,
        setting_value -> Text,
    }
}

diesel::table! {
    asset_taxonomy_assignments (id) {
        id -> Text,
        asset_id -> Text,
        taxonomy_id -> Text,
        category_id -> Text,
        weight -> Integer,
        source -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    assets (id) {
        id -> Text,
        kind -> Text,
        name -> Nullable<Text>,
        display_code -> Nullable<Text>,
        notes -> Nullable<Text>,
        metadata -> Nullable<Text>,
        is_active -> Integer,
        quote_mode -> Text,
        quote_ccy -> Text,
        instrument_type -> Nullable<Text>,
        instrument_symbol -> Nullable<Text>,
        instrument_exchange_mic -> Nullable<Text>,
        instrument_key -> Nullable<Text>,
        provider_config -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        expense_ratio -> Nullable<Double>,
    }
}

diesel::table! {
    brokers_sync_state (account_id, provider) {
        account_id -> Text,
        provider -> Text,
        checkpoint_json -> Nullable<Text>,
        last_attempted_at -> Nullable<Text>,
        last_successful_at -> Nullable<Text>,
        last_error -> Nullable<Text>,
        last_run_id -> Nullable<Text>,
        sync_status -> Text,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    contribution_limits (id) {
        id -> Text,
        group_name -> Text,
        contribution_year -> Integer,
        limit_amount -> Double,
        account_ids -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        start_date -> Nullable<Timestamp>,
        end_date -> Nullable<Timestamp>,
    }
}

diesel::table! {
    market_data_custom_providers (id) {
        id -> Text,
        code -> Text,
        name -> Text,
        description -> Text,
        enabled -> Bool,
        priority -> Integer,
        config -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    daily_account_valuation (id) {
        id -> Text,
        account_id -> Text,
        valuation_date -> Date,
        account_currency -> Text,
        base_currency -> Text,
        fx_rate_to_base -> Text,
        cash_balance -> Text,
        investment_market_value -> Text,
        total_value -> Text,
        cost_basis -> Text,
        net_contribution -> Text,
        calculated_at -> Text,
    }
}

diesel::table! {
    goals (id) {
        id -> Text,
        title -> Text,
        description -> Nullable<Text>,
        target_amount -> Double,
        goal_type -> Text,
        status_lifecycle -> Text,
        status_health -> Text,
        priority -> Integer,
        cover_image_key -> Nullable<Text>,
        currency -> Nullable<Text>,
        start_date -> Nullable<Text>,
        target_date -> Nullable<Text>,
        summary_current_value -> Nullable<Double>,
        summary_progress -> Nullable<Double>,
        projected_completion_date -> Nullable<Text>,
        projected_value_at_target_date -> Nullable<Double>,
        created_at -> Text,
        updated_at -> Text,
        summary_target_amount -> Nullable<Double>,
    }
}

diesel::table! {
    goal_plans (goal_id) {
        goal_id -> Text,
        plan_kind -> Text,
        planner_mode -> Nullable<Text>,
        settings_json -> Text,
        summary_json -> Text,
        version -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    goals_allocation (id) {
        id -> Text,
        goal_id -> Text,
        account_id -> Text,
        share_percent -> Double,
        tax_bucket -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    health_issue_dismissals (issue_id) {
        issue_id -> Text,
        dismissed_at -> Text,
        data_hash -> Text,
    }
}

diesel::table! {
    holdings_snapshots (id) {
        id -> Text,
        account_id -> Text,
        snapshot_date -> Date,
        currency -> Text,
        positions -> Text,
        cash_balances -> Text,
        cost_basis -> Text,
        net_contribution -> Text,
        calculated_at -> Text,
        net_contribution_base -> Text,
        cash_total_account_currency -> Text,
        cash_total_base_currency -> Text,
        source -> Text,
    }
}

diesel::table! {
    import_runs (id) {
        id -> Text,
        account_id -> Text,
        source_system -> Text,
        run_type -> Text,
        mode -> Text,
        status -> Text,
        started_at -> Text,
        finished_at -> Nullable<Text>,
        review_mode -> Text,
        applied_at -> Nullable<Text>,
        checkpoint_in -> Nullable<Text>,
        checkpoint_out -> Nullable<Text>,
        summary -> Nullable<Text>,
        warnings -> Nullable<Text>,
        error -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    market_data_providers (id) {
        id -> Text,
        name -> Text,
        description -> Text,
        url -> Nullable<Text>,
        priority -> Integer,
        enabled -> Bool,
        logo_filename -> Nullable<Text>,
        last_synced_at -> Nullable<Text>,
        last_sync_status -> Nullable<Text>,
        last_sync_error -> Nullable<Text>,
        provider_type -> Text,
        config -> Nullable<Text>,
    }
}

diesel::table! {
    platforms (id) {
        id -> Text,
        name -> Nullable<Text>,
        url -> Text,
        external_id -> Nullable<Text>,
        kind -> Text,
        website_url -> Nullable<Text>,
        logo_url -> Nullable<Text>,
    }
}

diesel::table! {
    quote_sync_state (asset_id) {
        asset_id -> Text,
        position_closed_date -> Nullable<Text>,
        last_synced_at -> Nullable<Text>,
        data_source -> Text,
        sync_priority -> Integer,
        error_count -> Integer,
        last_error -> Nullable<Text>,
        profile_enriched_at -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    quotes (id) {
        id -> Text,
        asset_id -> Text,
        day -> Text,
        source -> Text,
        open -> Nullable<Text>,
        high -> Nullable<Text>,
        low -> Nullable<Text>,
        close -> Text,
        adjclose -> Nullable<Text>,
        volume -> Nullable<Text>,
        currency -> Text,
        notes -> Nullable<Text>,
        created_at -> Text,
        timestamp -> Text,
    }
}

diesel::table! {
    sync_applied_events (event_id) {
        event_id -> Text,
        seq -> BigInt,
        entity -> Text,
        entity_id -> Text,
        applied_at -> Text,
    }
}

diesel::table! {
    sync_cursor (id) {
        id -> Integer,
        cursor -> BigInt,
        updated_at -> Text,
    }
}

diesel::table! {
    sync_device_config (device_id) {
        device_id -> Text,
        key_version -> Nullable<Integer>,
        trust_state -> Text,
        last_bootstrap_at -> Nullable<Text>,
        min_snapshot_created_at -> Nullable<Text>,
    }
}

diesel::table! {
    sync_engine_state (id) {
        id -> Integer,
        lock_version -> BigInt,
        last_push_at -> Nullable<Text>,
        last_pull_at -> Nullable<Text>,
        last_error -> Nullable<Text>,
        consecutive_failures -> Integer,
        next_retry_at -> Nullable<Text>,
        last_cycle_status -> Nullable<Text>,
        last_cycle_duration_ms -> Nullable<BigInt>,
    }
}

diesel::table! {
    sync_entity_metadata (entity, entity_id) {
        entity -> Text,
        entity_id -> Text,
        last_event_id -> Text,
        last_client_timestamp -> Text,
        last_op -> Text,
        last_seq -> BigInt,
    }
}

diesel::table! {
    sync_outbox (event_id) {
        event_id -> Text,
        entity -> Text,
        entity_id -> Text,
        op -> Text,
        client_timestamp -> Text,
        payload -> Text,
        payload_key_version -> Integer,
        sent -> Integer,
        status -> Text,
        retry_count -> Integer,
        next_retry_at -> Nullable<Text>,
        last_error -> Nullable<Text>,
        last_error_code -> Nullable<Text>,
        device_id -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    sync_table_state (table_name) {
        table_name -> Text,
        enabled -> Integer,
        last_snapshot_restore_at -> Nullable<Text>,
        last_incremental_apply_at -> Nullable<Text>,
    }
}

diesel::table! {
    tax_profiles (id) {
        id -> Text,
        jurisdiction -> Text,
        tax_residence_country -> Text,
        default_tax_regime -> Text,
        pfu_or_bareme_preference -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tax_events (id) {
        id -> Text,
        report_id -> Text,
        event_type -> Text,
        category -> Text,
        suggested_box -> Nullable<Text>,
        account_id -> Text,
        asset_id -> Nullable<Text>,
        activity_id -> Nullable<Text>,
        event_date -> Text,
        amount_currency -> Text,
        amount_local -> Nullable<Text>,
        amount_eur -> Nullable<Text>,
        taxable_amount_eur -> Nullable<Text>,
        expenses_eur -> Nullable<Text>,
        confidence -> Text,
        included -> Integer,
        notes -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tax_event_sources (id) {
        id -> Text,
        tax_event_id -> Text,
        source_type -> Text,
        source_id -> Text,
        description -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tax_lot_allocations (id) {
        id -> Text,
        tax_event_id -> Text,
        source_activity_id -> Text,
        quantity -> Text,
        acquisition_date -> Text,
        cost_basis_eur -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tax_issues (id) {
        id -> Text,
        report_id -> Text,
        severity -> Text,
        code -> Text,
        message -> Text,
        account_id -> Nullable<Text>,
        activity_id -> Nullable<Text>,
        tax_event_id -> Nullable<Text>,
        resolved_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tax_documents (id) {
        id -> Text,
        report_id -> Text,
        document_type -> Text,
        filename -> Text,
        mime_type -> Nullable<Text>,
        sha256 -> Text,
        encrypted_content -> Text,
        encryption_key_ref -> Text,
        size_bytes -> Integer,
        uploaded_at -> Timestamp,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tax_document_extractions (id) {
        id -> Text,
        document_id -> Text,
        method -> Text,
        status -> Text,
        consent_granted -> Integer,
        raw_text_preview -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    extracted_tax_fields (id) {
        id -> Text,
        extraction_id -> Text,
        field_key -> Text,
        label -> Text,
        mapped_category -> Nullable<Text>,
        value_text -> Nullable<Text>,
        amount_eur -> Nullable<Text>,
        confidence -> Double,
        status -> Text,
        confirmed_amount_eur -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tax_reconciliation_entries (id) {
        id -> Text,
        report_id -> Text,
        category -> Text,
        suggested_box -> Nullable<Text>,
        app_amount_eur -> Nullable<Text>,
        document_amount_eur -> Nullable<Text>,
        selected_amount_eur -> Nullable<Text>,
        delta_eur -> Nullable<Text>,
        status -> Text,
        notes -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    tax_year_reports (id) {
        id -> Text,
        tax_year -> Integer,
        jurisdiction -> Text,
        status -> Text,
        rule_pack_version -> Text,
        base_currency -> Text,
        generated_at -> Nullable<Timestamp>,
        finalized_at -> Nullable<Timestamp>,
        assumptions_json -> Text,
        summary_json -> Text,
        parent_report_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    taxonomies (id) {
        id -> Text,
        name -> Text,
        color -> Text,
        description -> Nullable<Text>,
        is_system -> Integer,
        is_single_select -> Integer,
        sort_order -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    taxonomy_categories (id, taxonomy_id) {
        id -> Text,
        taxonomy_id -> Text,
        parent_id -> Nullable<Text>,
        name -> Text,
        key -> Text,
        color -> Text,
        description -> Nullable<Text>,
        sort_order -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::joinable!(account_tax_profiles -> accounts (account_id));
diesel::joinable!(accounts -> platforms (platform_id));
diesel::joinable!(activities -> accounts (account_id));
diesel::joinable!(activities -> assets (asset_id));
diesel::joinable!(activities -> import_runs (import_run_id));
diesel::joinable!(ai_messages -> ai_threads (thread_id));
diesel::joinable!(ai_thread_tags -> ai_threads (thread_id));
diesel::joinable!(asset_taxonomy_assignments -> assets (asset_id));
diesel::joinable!(brokers_sync_state -> accounts (account_id));
diesel::joinable!(brokers_sync_state -> import_runs (last_run_id));
diesel::joinable!(goals_allocation -> accounts (account_id));
diesel::joinable!(goal_plans -> goals (goal_id));
diesel::joinable!(goals_allocation -> goals (goal_id));
diesel::joinable!(import_runs -> accounts (account_id));
diesel::joinable!(quotes -> assets (asset_id));
diesel::joinable!(tax_documents -> tax_year_reports (report_id));
diesel::joinable!(tax_document_extractions -> tax_documents (document_id));
diesel::joinable!(extracted_tax_fields -> tax_document_extractions (extraction_id));
diesel::joinable!(tax_events -> tax_year_reports (report_id));
diesel::joinable!(tax_event_sources -> tax_events (tax_event_id));
diesel::joinable!(tax_lot_allocations -> tax_events (tax_event_id));
diesel::joinable!(tax_reconciliation_entries -> tax_year_reports (report_id));
diesel::joinable!(taxonomy_categories -> taxonomies (taxonomy_id));

diesel::joinable!(import_account_templates -> import_templates (template_id));

diesel::allow_tables_to_appear_in_same_query!(
    import_account_templates,
    account_tax_profiles,
    accounts,
    activities,
    ai_messages,
    ai_thread_tags,
    ai_threads,
    app_settings,
    asset_taxonomy_assignments,
    assets,
    brokers_sync_state,
    contribution_limits,
    market_data_custom_providers,
    daily_account_valuation,
    goal_plans,
    goals,
    goals_allocation,
    health_issue_dismissals,
    holdings_snapshots,
    import_templates,
    import_runs,
    market_data_providers,
    platforms,
    quote_sync_state,
    quotes,
    sync_applied_events,
    sync_cursor,
    sync_device_config,
    sync_engine_state,
    sync_entity_metadata,
    sync_outbox,
    sync_table_state,
    extracted_tax_fields,
    tax_document_extractions,
    tax_documents,
    tax_events,
    tax_event_sources,
    tax_issues,
    tax_lot_allocations,
    tax_profiles,
    tax_reconciliation_entries,
    tax_year_reports,
    taxonomies,
    taxonomy_categories,
);
