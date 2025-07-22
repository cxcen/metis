use crate::types::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::str::FromStr;

/// Metis 路由算法的数学工具
/// 
/// 提供各种数学计算功能，包括：
/// - 汇率权重计算
/// - 价格影响计算
/// - 分割比率计算
/// - 滑点边界计算
pub struct MathUtils;

impl MathUtils {
    /// 计算汇率的负对数作为 Bellman-Ford 权重
    /// 
    /// # 参数
    /// * `exchange_rate` - 汇率值（大于 0 的小数）
    /// 
    /// # 返回值
    /// * `f64` - 负对数权重，用于 Bellman-Ford 算法
    pub fn calculate_edge_weight(exchange_rate: Decimal) -> f64 {
        -f64::ln(exchange_rate.to_string().parse::<f64>().unwrap_or(1.0))
    }

    /// 考虑费用和价格影响计算有效汇率
    /// 
    /// # 参数
    /// * `base_rate` - 基础汇率
    /// * `fee_rate` - 费用率（0-1 之间的小数）
    /// * `price_impact` - 价格影响（0-1 之间的小数）
    /// 
    /// # 返回值
    /// * `Decimal` - 考虑费用和价格影响后的有效汇率
    pub fn calculate_effective_rate(
        base_rate: Decimal,
        fee_rate: Decimal,
        price_impact: Decimal,
    ) -> Decimal {
        base_rate * (dec!(1) - fee_rate) * (dec!(1) - price_impact)
    }

    /// 计算路由分割的最优分割比率
    /// 
    /// # 参数
    /// * `num_splits` - 分割数量（1-10）
    /// 
    /// # 返回值
    /// * `Vec<Decimal>` - 分割比率列表，总和为 1.0
    pub fn calculate_split_ratios(num_splits: usize) -> Vec<Decimal> {
        match num_splits {
            1 => vec![dec!(1.0)],
            2 => vec![dec!(0.6), dec!(0.4)],
            3 => vec![dec!(0.5), dec!(0.3), dec!(0.2)],
            4 => vec![dec!(0.4), dec!(0.25), dec!(0.2), dec!(0.15)],
            _ => {
                // 对于更多分割，使用指数衰减
                let mut ratios = Vec::new();
                let base_ratio = dec!(0.4);
                for i in 0..num_splits {
                    let decay_factor = dec!(0.7).to_string().parse::<f64>().unwrap_or(0.7).powi(i as i32);
                    let ratio = base_ratio * Decimal::from_str(&decay_factor.to_string()).unwrap_or(dec!(0.7));
                    ratios.push(ratio);
                }
                // 标准化为总和为 1.0
                let total: Decimal = ratios.iter().sum();
                ratios.iter().map(|&r| r / total).collect()
            }
        }
    }

    /// 使用恒定乘积 AMM 公式计算价格影响
    /// 
    /// # 参数
    /// * `input_amount` - 输入代币数量
    /// * `reserve_in` - 输入代币的储备量
    /// * `reserve_out` - 输出代币的储备量
    /// 
    /// # 返回值
    /// * `Decimal` - 价格影响（0-1 之间的小数）
    pub fn calculate_amm_price_impact(
        input_amount: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
    ) -> Decimal {
        // 恒定乘积公式：(x + dx) * (y - dy) = x * y
        // dy = y * dx / (x + dx)
        let _output_amount = reserve_out * input_amount / (reserve_in + input_amount);
        let price_impact = input_amount / (reserve_in + input_amount);
        price_impact
    }

    /// 计算滑点容差边界
    /// 
    /// # 参数
    /// * `expected_amount` - 预期输出数量
    /// * `slippage_tolerance` - 滑点容差（0-1 之间的小数）
    /// 
    /// # 返回值
    /// * `(Decimal, Decimal)` - (最小数量, 最大数量) 的元组
    pub fn calculate_slippage_bounds(
        expected_amount: Decimal,
        slippage_tolerance: Decimal,
    ) -> (Decimal, Decimal) {
        let min_amount = expected_amount * (dec!(1) - slippage_tolerance);
        let max_amount = expected_amount * (dec!(1) + slippage_tolerance);
        (min_amount, max_amount)
    }

    /// 计算路由的 gas 成本
    /// 
    /// # 参数
    /// * `num_dex_interactions` - DEX 交互次数
    /// * `gas_price` - 每单位 gas 的价格
    /// * `base_gas_per_dex` - 每次 DEX 交互的基础 gas 消耗
    /// 
    /// # 返回值
    /// * `Decimal` - 总 gas 成本
    pub fn calculate_gas_cost(
        num_dex_interactions: usize,
        gas_price: Decimal,
        base_gas_per_dex: Decimal,
    ) -> Decimal {
        let total_gas = base_gas_per_dex * Decimal::from(num_dex_interactions);
        total_gas * gas_price
    }
}

/// 用于显示路由信息的格式化工具
/// 
/// 提供各种格式化功能，包括：
/// - 百分比格式化
/// - 货币格式化
/// - 路由摘要格式化
/// - 分割路由摘要格式化
pub struct FormatUtils;

impl FormatUtils {
    /// 将小数格式化为百分比
    /// 
    /// # 参数
    /// * `value` - 要格式化的值（0-1 之间的小数）
    /// 
    /// # 返回值
    /// * `String` - 格式化的百分比字符串，如 "12.34%"
    pub fn format_percentage(value: Decimal) -> String {
        format!("{:.2}%", value * dec!(100))
    }

    /// 将小数格式化为货币
    /// 
    /// # 参数
    /// * `value` - 要格式化的数值
    /// * `symbol` - 货币符号，如 "USDC", "SOL"
    /// 
    /// # 返回值
    /// * `String` - 格式化的货币字符串，如 "1000.00 USDC"
    pub fn format_currency(value: Decimal, symbol: &str) -> String {
        format!("{} {}", value, symbol)
    }

    /// 格式化路由摘要用于显示
    /// 
    /// # 参数
    /// * `route` - 要格式化的路由
    /// 
    /// # 返回值
    /// * `String` - 格式化的路由摘要
    pub fn format_route_summary(route: &Route) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("路由: {} -> {}\n", 
                                 route.segments.first().unwrap().from_token.symbol,
                                 route.segments.last().unwrap().to_token.symbol));
        summary.push_str(&format!("输入: {}\n", 
                                 Self::format_currency(route.total_input_amount, 
                                 &route.segments.first().unwrap().from_token.symbol)));
        summary.push_str(&format!("输出: {}\n", 
                                 Self::format_currency(route.total_output_amount,
                                 &route.segments.last().unwrap().to_token.symbol)));
        summary.push_str(&format!("有效汇率: {}\n", route.effective_rate));
        summary.push_str(&format!("价格影响: {}\n", 
                                 Self::format_percentage(route.price_impact)));
        summary.push_str(&format!("Gas 成本: {}\n", 
                                 Self::format_currency(route.gas_estimate, "SOL")));
        summary.push_str(&format!("跳数: {}", route.segments.len()));
        summary
    }

    /// 格式化分割路由摘要
    /// 
    /// # 参数
    /// * `split_route` - 要格式化的分割路由
    /// 
    /// # 返回值
    /// * `String` - 格式化的分割路由摘要
    pub fn format_split_route_summary(split_route: &SplitRoute) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("分割路由: {} 个路由\n", split_route.routes.len()));
        summary.push_str(&format!("总输入: {}\n", split_route.total_input_amount));
        summary.push_str(&format!("总输出: {}\n", split_route.total_output_amount));
        summary.push_str(&format!("有效汇率: {}\n", split_route.effective_rate));
        summary.push_str(&format!("总价格影响: {}\n", 
                                 Self::format_percentage(split_route.price_impact)));
        summary.push_str(&format!("总 Gas 成本: {}\n", 
                                 Self::format_currency(split_route.gas_estimate, "SOL")));
        
        for (i, route) in split_route.routes.iter().enumerate() {
            summary.push_str(&format!("\n路由 {}: {} -> {} ({}%)", 
                                     i + 1,
                                     route.segments.first().unwrap().from_token.symbol,
                                     route.segments.last().unwrap().to_token.symbol,
                                     Self::format_percentage(route.split_ratio.unwrap_or(dec!(0)))));
        }
        summary
    }
}

/// 路由请求和响应的验证工具
/// 
/// 提供各种验证功能，包括：
/// - 路由请求参数验证
/// - 路由响应验证
/// - 数据完整性检查
pub struct ValidationUtils;

impl ValidationUtils {
    /// 验证路由请求参数
    /// 
    /// # 参数
    /// * `request` - 要验证的路由请求
    /// 
    /// # 返回值
    /// * `Result<(), String>` - 验证结果，错误时返回错误信息
    pub fn validate_route_request(request: &RouteRequest) -> Result<(), String> {
        if request.input_amount <= dec!(0) {
            return Err("输入数量必须为正数".to_string());
        }

        if request.slippage_tolerance <= dec!(0) || request.slippage_tolerance >= dec!(1) {
            return Err("滑点容差必须在 0 和 1 之间".to_string());
        }

        if request.max_iterations == 0 {
            return Err("最大迭代次数必须大于 0".to_string());
        }

        if request.input_token == request.output_token {
            return Err("输入和输出代币必须不同".to_string());
        }

        Ok(())
    }

    /// 验证路由响应
    /// 
    /// # 参数
    /// * `response` - 要验证的路由响应
    /// 
    /// # 返回值
    /// * `Result<(), String>` - 验证结果，错误时返回错误信息
    pub fn validate_route_response(response: &RouteResponse) -> Result<(), String> {
        if response.route.is_none() && response.split_route.is_none() {
            return Err("响应中未找到路由".to_string());
        }

        if let Some(route) = &response.route {
            if route.segments.is_empty() {
                return Err("路由没有段".to_string());
            }

            if route.total_input_amount <= dec!(0) {
                return Err("路由输入数量无效".to_string());
            }

            if route.total_output_amount <= dec!(0) {
                return Err("路由输出数量无效".to_string());
            }
        }

        if let Some(split_route) = &response.split_route {
            if split_route.routes.is_empty() {
                return Err("分割路由没有路由".to_string());
            }

            if split_route.total_input_amount <= dec!(0) {
                return Err("分割路由输入数量无效".to_string());
            }

            if split_route.total_output_amount <= dec!(0) {
                return Err("分割路由输出数量无效".to_string());
            }
        }

        Ok(())
    }
}

/// 性能测量工具
/// 
/// 提供各种性能测量功能，包括：
/// - 执行时间测量
/// - 吞吐量计算
/// - 平均时间计算
pub struct PerformanceUtils;

impl PerformanceUtils {
    /// 测量函数的执行时间
    /// 
    /// # 参数
    /// * `func` - 要测量的异步函数
    /// 
    /// # 返回值
    /// * `(T, std::time::Duration)` - 函数结果和执行时间的元组
    pub async fn measure_execution_time<F, T>(func: F) -> (T, std::time::Duration)
    where
        F: std::future::Future<Output = T>,
    {
        let start = std::time::Instant::now();
        let result = func.await;
        let duration = start.elapsed();
        (result, duration)
    }

    /// 计算吞吐量（每秒操作数）
    /// 
    /// # 参数
    /// * `operations` - 操作数量
    /// * `duration` - 执行时间
    /// 
    /// # 返回值
    /// * `f64` - 每秒操作数
    pub fn calculate_throughput(operations: usize, duration: std::time::Duration) -> f64 {
        let duration_secs = duration.as_secs_f64();
        if duration_secs > 0.0 {
            operations as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// 计算平均执行时间
    /// 
    /// # 参数
    /// * `times` - 执行时间列表
    /// 
    /// # 返回值
    /// * `std::time::Duration` - 平均执行时间
    pub fn calculate_average_time(times: &[std::time::Duration]) -> std::time::Duration {
        if times.is_empty() {
            return std::time::Duration::ZERO;
        }

        let total_nanos: u128 = times.iter().map(|d| d.as_nanos()).sum();
        let avg_nanos = total_nanos / times.len() as u128;
        std::time::Duration::from_nanos(avg_nanos as u64)
    }
}

/// 管理报价和路由缓存的缓存工具
/// 
/// 提供各种缓存管理功能，包括：
/// - 缓存键生成
/// - 缓存过期检查
/// - 缓存统计
pub struct CacheUtils;

impl CacheUtils {
    /// 为路由请求生成缓存键
    /// 
    /// # 参数
    /// * `request` - 路由请求
    /// 
    /// # 返回值
    /// * `String` - 缓存键
    pub fn generate_route_cache_key(request: &RouteRequest) -> String {
        format!("route:{}:{}:{}:{}:{}", 
                request.input_token,
                request.output_token,
                request.input_amount,
                request.slippage_tolerance,
                request.enable_split_routes)
    }

    /// 为报价请求生成缓存键
    /// 
    /// # 参数
    /// * `request` - 报价请求
    /// 
    /// # 返回值
    /// * `String` - 缓存键
    pub fn generate_quote_cache_key(request: &QuoteRequest) -> String {
        format!("quote:{}:{}:{}:{}", 
                request.dex_platform,
                request.input_token,
                request.output_token,
                request.amount)
    }

    /// 检查缓存条目是否过期
    /// 
    /// # 参数
    /// * `created_at` - 缓存条目创建时间
    /// * `ttl_seconds` - 生存时间（秒）
    /// 
    /// # 返回值
    /// * `bool` - 是否已过期
    pub fn is_cache_expired(created_at: std::time::Instant, ttl_seconds: u64) -> bool {
        created_at.elapsed().as_secs() > ttl_seconds
    }
} 