use super::collect::PersonDay;
use bean::database::{
    BeanDBPool, CreateCategory, CreateEntry, create_category, create_entry, get_category_by_name,
};
use cadence_core::error::CadenceError;
use chrono::{DateTime, Utc};

/// Store one person's `(title, summary)` under `entry_date`, creating the
/// per-person category on demand.
pub async fn store_summary(
    bean: &BeanDBPool,
    person: &PersonDay,
    title: &str,
    summary: &str,
    entry_date: DateTime<Utc>,
) -> Result<(), CadenceError> {
    // One category per individual, created on demand.
    let category_id = match get_category_by_name(bean, person.username.clone())
        .await
        .map_err(bean_err)?
    {
        Some(c) => c.id,
        None => create_category(
            bean,
            CreateCategory {
                name: person.username.clone(),
                description: format!("Daily Dilly chat summaries for {}", person.display),
            },
        )
        .await
        .map_err(bean_err)?,
    };

    create_entry(
        bean,
        CreateEntry {
            category_id,
            name: title.to_string(),
            content: summary.to_string(),
            entry_date,
        },
    )
    .await
    .map_err(bean_err)?;
    Ok(())
}

fn bean_err(e: bean::error::BeanError) -> CadenceError {
    CadenceError::Channel(e.to_string())
}
