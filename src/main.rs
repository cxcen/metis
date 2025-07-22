mod types;
mod graph;
mod routing;
mod quote;
mod utils;

use anyhow::Result;
use log::{info, warn};
use types::*;
use routing::MetisRouter;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    info!("🚀 启动 Metis DEX 聚合路由器");
    
    // Metis 路由算法的使用示例
    let mut router = MetisRouter::new();
    router.initialize();
    
    // 示例：用 1000 USDC 寻找从 USDC 到 SOL 的最优路由
    let request = RouteRequest {
        input_token: "USDC".to_string(),
        output_token: "SOL".to_string(),
        input_amount: rust_decimal_macros::dec!(1000.0),
        slippage_tolerance: rust_decimal_macros::dec!(0.005), // 0.5%
        max_iterations: 5,
        enable_split_routes: true,
        max_splits: Some(3),
    };
    
    match router.find_optimal_route(request).await {
        Ok(response) => {
            if response.route.is_some() || response.split_route.is_some() {
                info!("✅ 找到最优路由:");
                println!("{}", serde_json::to_string_pretty(&response)?);
                
                // 如果找到路由，显示额外分析
                if let Some(route) = &response.route {
                    let analysis = router.analyze_route(route);
                    println!("\n📊 路由分析:");
                    println!("总跳数: {}", analysis.total_hops);
                    println!("平均价格影响: {:.2}%", analysis.avg_price_impact * rust_decimal_macros::dec!(100));
                    println!("总费用: {:.6} USDC", analysis.total_fees);
                    println!("效率分数: {:.2}", analysis.efficiency_score);
                    
                    if !analysis.recommendations.is_empty() {
                        println!("\n💡 建议:");
                        for rec in &analysis.recommendations {
                            println!("- {}", rec);
                        }
                    }
                }
            } else {
                warn!("❌ 未找到路由");
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
        }
        Err(e) => {
            eprintln!("❌ 寻找路由失败: {}", e);
        }
    }
    
    Ok(())
}
