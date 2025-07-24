use screenerbot::summary::display_current_bot_summary;

#[tokio::main]
async fn main() {
    println!("🤖 ScreenerBot - Enhanced Summary Display Demo\n");

    // Display the enhanced bot summary
    display_current_bot_summary().await;

    println!("Demo completed! ✅");
}
