use chrono::Utc;
use screenerbot::strategies::{
  db::{get_all_strategies, get_enabled_strategies, get_strategy, insert_strategy},
  engine::{EngineConfig, StrategyEngine},
  types::*,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), String> {
 println!("Strategy System Debug Tool");
  println!("================================\n");

  // Parse command-line arguments
  let args: Vec<String> = std::env::args().collect();
  let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

  match command {
 "init"=> init_db(),
 "list"=> list_strategies(),
 "create-example"=> create_example_strategy().await,
 "validate"=> validate_strategy_by_id(&args).await,
 "test-evaluate"=> test_evaluation().await,
 "schemas"=> print_condition_schemas().await,
 "help"=> {
      print_help();
      Ok(())
    }
    _ => {
      println!("Unknown command: {}", command);
      print_help();
      Ok(())
    }
  }
}

fn print_help() {
  println!("Usage: debug_strategies <command>");
  println!("\nCommands:");
 println!("init - Initialize strategies database");
 println!("list - List all strategies");
 println!("create-example - Create example strategies");
 println!("validate <id> - Validate a strategy by ID");
 println!("test-evaluate - Test strategy evaluation");
 println!("schemas - Print all condition schemas");
 println!("help - Show this help message");
}

fn init_db() -> Result<(), String> {
  println!("Initializing strategies database...");
  screenerbot::strategies::db::init_strategies_db()?;
 println!("Database initialized successfully");
  Ok(())
}

fn list_strategies() -> Result<(), String> {
  let strategies = get_all_strategies()?;

  if strategies.is_empty() {
 println!("No strategies found. Use 'create-example'to create some.");
    return Ok(());
  }

  println!("Found {} strategies:\n", strategies.len());

  for strategy in strategies {
 println!("Strategy: {}", strategy.name);
 println!("ID: {}", strategy.id);
 println!("Type: {}", strategy.strategy_type);
 println!("Enabled: {}", strategy.enabled);
 println!("Priority: {}", strategy.priority);
    if let Some(desc) = &strategy.description {
 println!("Description: {}", desc);
    }
    println!();
  }

  Ok(())
}

async fn create_example_strategy() -> Result<(), String> {
  println!("Creating example entry strategy...");

  // Example: Simple price threshold entry strategy
  let mut parameters = HashMap::new();
  let price_param = Parameter {
    value: serde_json::json!(0.00001),
    default: serde_json::json!(0.0),
    constraints: None,
  };
  parameters.insert("value".to_string(), price_param);

  let comparison_param = Parameter {
    value: serde_json::json!("ABOVE"),
    default: serde_json::json!("ABOVE"),
    constraints: None,
  };
  parameters.insert("comparison".to_string(), comparison_param);

  let condition = Condition {
    condition_type: "PriceThreshold".to_string(),
    parameters,
  };

  let rule_tree = RuleTree::leaf(condition);

  let strategy = Strategy {
    id: "example-price-threshold".to_string(),
    name: "Simple Price Threshold Entry".to_string(),
    description: Some("Enter when price is above 0.00001 SOL".to_string()),
    strategy_type: StrategyType::Entry,
    enabled: true,
    priority: 10,
    rules: rule_tree,
    parameters: HashMap::new(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
    author: Some("debug_tool".to_string()),
    version: 1,
  };

  insert_strategy(&strategy)?;
 println!("Created strategy: {}", strategy.name);

  // Example 2: Liquidity + Price movement strategy
  println!("\nCreating complex AND strategy...");

  let liquidity_condition = Condition {
    condition_type: "LiquidityDepth".to_string(),
    parameters: {
      let mut params = HashMap::new();
      params.insert(
        "threshold".to_string(),
        Parameter {
          value: serde_json::json!(50.0),
          default: serde_json::json!(50.0),
          constraints: None,
        },
      );
      params.insert(
        "comparison".to_string(),
        Parameter {
          value: serde_json::json!("GREATER_THAN"),
          default: serde_json::json!("GREATER_THAN"),
          constraints: None,
        },
      );
      params
    },
  };

  let price_movement_condition = Condition {
    condition_type: "PriceMovement".to_string(),
    parameters: {
      let mut params = HashMap::new();
      params.insert(
        "timeframe".to_string(),
        Parameter {
          value: serde_json::json!("5m"),
          default: serde_json::json!("5m"),
          constraints: None,
        },
      );
      params.insert(
        "percentage".to_string(),
        Parameter {
          value: serde_json::json!(5.0),
          default: serde_json::json!(5.0),
          constraints: None,
        },
      );
      params.insert(
        "direction".to_string(),
        Parameter {
          value: serde_json::json!("UP"),
          default: serde_json::json!("UP"),
          constraints: None,
        },
      );
      params
    },
  };

  let and_rule = RuleTree::branch(
    LogicalOperator::And,
    vec![
      RuleTree::leaf(liquidity_condition),
      RuleTree::leaf(price_movement_condition),
    ],
  );

  let strategy2 = Strategy {
    id: "example-momentum-with-liquidity".to_string(),
    name: "Momentum Entry with Liquidity Check".to_string(),
    description: Some(
      "Enter when price moves up 5% in 5min AND liquidity > 50 SOL".to_string(),
    ),
    strategy_type: StrategyType::Entry,
    enabled: true,
    priority: 20,
    rules: and_rule,
    parameters: HashMap::new(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
    author: Some("debug_tool".to_string()),
    version: 1,
  };

  insert_strategy(&strategy2)?;
 println!("Created strategy: {}", strategy2.name);

  Ok(())
}

async fn validate_strategy_by_id(args: &[String]) -> Result<(), String> {
  let strategy_id = args.get(2).ok_or("Usage: validate <strategy_id>")?;

  println!("Validating strategy: {}", strategy_id);

  let strategy =
    get_strategy(strategy_id)?.ok_or_else(|| format!("Strategy not found: {}", strategy_id))?;

  println!("Strategy: {}", strategy.name);
  println!("Type: {}", strategy.strategy_type);

  let engine = StrategyEngine::new(EngineConfig::default());

  match engine.validate_strategy(&strategy) {
    Ok(_) => {
 println!("Strategy is valid");
      Ok(())
    }
    Err(e) => {
 println!("Strategy validation failed: {}", e);
      Err(e)
    }
  }
}

async fn test_evaluation() -> Result<(), String> {
  println!("Testing strategy evaluation...\n");

  // Initialize engine
  let engine = StrategyEngine::new(EngineConfig::default());

  // Get enabled strategies
  let strategies = get_enabled_strategies(StrategyType::Entry)?;

  if strategies.is_empty() {
 println!("No enabled entry strategies found. Use 'create-example'first.");
    return Ok(());
  }

  println!("Found {} enabled entry strategies\n", strategies.len());

  // Create test context
  let context = EvaluationContext {
    token_mint: "TEST1234567890".to_string(),
    current_price: Some(0.00002),
    position_data: None,
    market_data: Some(MarketData {
      liquidity_sol: Some(100.0),
      volume_24h: Some(1000.0),
      market_cap: Some(50000.0),
      holder_count: Some(500),
      token_age_hours: Some(24.0),
    }),
    ohlcv_data: None,
  };

  // Evaluate each strategy
  for strategy in strategies {
    println!("Evaluating: {}", strategy.name);

    match engine.evaluate_strategy(&strategy, &context).await {
      Ok(result) => {
        println!(
 "Result: {}",
          if result.result {
 "SIGNAL"
          } else {
 "NO SIGNAL"
          }
        );
 println!("Execution time: {}ms", result.execution_time_ms);
 println!("Confidence: {:.2}", result.confidence);
      }
      Err(e) => {
 println!("Error: {}", e);
      }
    }
    println!();
  }

  Ok(())
}

async fn print_condition_schemas() -> Result<(), String> {
  println!("Available Condition Types and Schemas:\n");

  let engine = StrategyEngine::new(EngineConfig::default());
  let registry = engine.get_condition_registry();
  let schemas = registry.get_all_schemas();

  println!("{}", serde_json::to_string_pretty(&schemas).unwrap());

  Ok(())
}
