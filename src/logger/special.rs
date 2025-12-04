//! Special logging functions for specific use cases

use super::tags::LogTag;
use crate::logger;
use colored::*;

/// Enhanced logging function for price changes with comprehensive position details
///
/// Displays price changes with:
/// - Symbol and price change with directional indicators
/// - Pool vs API price comparison
/// - Current P&L if position is open
/// - Pool type and address information
pub fn log_price_change(
  mint: &str,
  symbol: &str,
  old_price: f64,
  new_price: f64,
  price_source: &str,
  pool_type: Option<&str>,
  pool_address: Option<&str>,
  api_price: Option<f64>,
  current_pnl: Option<(f64, f64)>, // (pnl_sol, pnl_percent)
) {
  let price_change = new_price - old_price;
  let price_change_percent = if old_price != 0.0 {
    (price_change / old_price) * 100.0
  } else {
    0.0
  };

  // Price direction indicator and color
  let (emoji, price_color) = if price_change > 0.0 {
    ("", "green")
  } else if price_change < 0.0 {
    ("", "red")
  } else {
    ("", "yellow")
  };

  // Format pool type
  let formatted_pool_type = pool_type
    .map(|pt| {
      if pt.chars().any(|c| c.is_uppercase()) && pt.contains(' ') {
        pt.to_string()
      } else {
        pt.split('-')
          .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
              None => String::new(),
              Some(first) => {
                first.to_uppercase().collect::<String>()
                  + &chars.as_str().to_uppercase()
              }
            }
          })
          .collect::<Vec<String>>()
          .join(" ")
      }
    })
    .unwrap_or_else(|| "Unknown".to_string());

  // Line 1: Symbol, price change, and P&L
  let mut line1_parts = Vec::new();

  let price_part = format!(
    "{} {} {:.10} SOL ( {}SOL, {} % )",
    emoji,
    format!("{}", symbol).bold(),
    match price_color {
 "green"=> format!("{:.10}", new_price).green().bold(),
 "red"=> format!("{:.10}", new_price).red().bold(),
      _ => format!("{:.10}", new_price).white().bold(),
    },
    match price_color {
 "green"=> format!("+{:.10} ", price_change).green().bold(),
 "red"=> format!("{:.10} ", price_change).red().bold(),
      _ => format!("+{:.10} ", 0.0).white().bold(),
    },
    match price_color {
 "green"=> format!("+{:.2}", price_change_percent).green().bold(),
 "red"=> format!("{:.2}", price_change_percent).red().bold(),
      _ => format!("+{:.2}", 0.0).white().bold(),
    }
  );
  line1_parts.push(price_part);

  // P&L section
  if let Some((pnl_sol, pnl_percent)) = current_pnl {
    let pnl_text = if pnl_percent > 0.0 {
      format!(
 "P&L: {} SOL ( {} % )",
        format!("+{:.6}", pnl_sol).green().bold(),
        format!("+{:.2}", pnl_percent).green().bold()
      )
    } else if pnl_percent < 0.0 {
      format!(
 "P&L: {} SOL ( {} % )",
        format!("{:.6}", pnl_sol).red().bold(),
        format!("{:.2}", pnl_percent).red().bold()
      )
    } else {
      format!(
 "P&L: {} SOL ( {} % )",
        format!("±{:.6}", 0.0).white().bold(),
        format!("±{:.2}", 0.0).white().bold()
      )
    };
    line1_parts.push(pnl_text);
  }

 let line1 = line1_parts.join("");

  // Line 2: Pool vs API comparison and pool details
  let mut line2_parts = Vec::new();

 if price_source == "pool"{
    if let Some(api_price_val) = api_price {
      let diff = new_price - api_price_val;
      let diff_percent = if api_price_val != 0.0 {
        (diff / api_price_val) * 100.0
      } else {
        0.0
      };

      line2_parts.push(format!(
 "Pool: {}",
        format!("{:.10}", new_price).white().bold()
      ));
      line2_parts.push(format!(
 "API: {}",
        format!("{:.10}", api_price_val).white().bold()
      ));

      let diff_text = if diff > 0.0 {
        format!(
          "( Pool {} % )",
          format!("+{:.2}", diff_percent).green().bold()
        )
      } else if diff < 0.0 {
        format!("( Pool {} % )", format!("{:.2}", diff_percent).red().bold())
      } else {
        "(Perfect Match)".white().to_string()
      };
      line2_parts.push(diff_text);
    } else {
      line2_parts.push(
 format!("{} Pool", formatted_pool_type)
          .dimmed()
          .to_string(),
      );
    }
  } else {
 line2_parts.push("API Price".dimmed().to_string());
  }

  // Pool details
  if pool_address.is_some() {
    line2_parts.push(
      format!("[ {} ]", formatted_pool_type)
        .bright_yellow()
        .to_string(),
    );
  }

  let line2 = line2_parts
    .into_iter()
    .map(|part| part.to_string())
    .collect::<Vec<String>>()
 .join("");

  // Combine both lines
  let combined_message = format!("{}\n{}", line1, line2);

  // Log using the standard info function
  logger::info(LogTag::Positions, &combined_message);
}
