use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 路由图中的代币表示
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Token {
    /// 代币符号，如 "USDC", "SOL"
    pub symbol: String,
    /// 代币在区块链上的地址
    pub address: String,
    /// 代币的小数位数，如 USDC 为 6，SOL 为 9
    pub decimals: u8,
}

/// DEX 平台信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPlatform {
    /// DEX 平台名称，如 "Raydium", "Orca"
    pub name: String,
    /// DEX 平台在区块链上的地址
    pub address: String,
    /// 交易费用率，例如：0.003 表示 0.3%
    pub fee_rate: Decimal,
}

/// 表示具有流动性约束的交易对边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// 源代币（输入代币）
    pub from_token: Token,
    /// 目标代币（输出代币）
    pub to_token: Token,
    /// 提供此交易对的 DEX 平台
    pub dex_platform: DexPlatform,
    /// 当前汇率（1 个输入代币可兑换的输出代币数量）
    pub exchange_rate: Decimal,
    /// 该交易对的可用流动性总量
    pub liquidity: Decimal,
    /// 该交易对能承受的最大单笔交易规模
    pub max_trade_size: Decimal,
    /// 该交易对的最小交易规模
    pub min_trade_size: Decimal,
    /// Bellman-Ford 算法的权重，值为 -log(exchange_rate)
    pub weight: f64,
}

/// 路由中的路径段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSegment {
    /// 该段的输入代币
    pub from_token: Token,
    /// 该段的输出代币
    pub to_token: Token,
    /// 执行该段交易的 DEX 平台
    pub dex_platform: DexPlatform,
    /// 该段的输入数量
    pub input_amount: Decimal,
    /// 该段的输出数量
    pub output_amount: Decimal,
    /// 该段的有效汇率
    pub exchange_rate: Decimal,
    /// 该段的价格影响（滑点）
    pub price_impact: Decimal,
}

/// 从输入到输出代币的完整路由
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// 路由中的所有路径段
    pub segments: Vec<PathSegment>,
    /// 整个路由的总输入数量
    pub total_input_amount: Decimal,
    /// 整个路由的总输出数量
    pub total_output_amount: Decimal,
    /// 整个路由的有效汇率（总输出/总输入）
    pub effective_rate: Decimal,
    /// 整个路由的总价格影响
    pub price_impact: Decimal,
    /// 执行该路由的预估 gas 成本
    pub gas_estimate: Decimal,
    /// 该路由在分割路由中的占比（用于分割路由）
    pub split_ratio: Option<Decimal>,
}

/// 分割路由配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitRoute {
    /// 分割路由中包含的所有子路由
    pub routes: Vec<Route>,
    /// 分割路由的总输入数量
    pub total_input_amount: Decimal,
    /// 分割路由的总输出数量
    pub total_output_amount: Decimal,
    /// 分割路由的有效汇率
    pub effective_rate: Decimal,
    /// 分割路由的总价格影响
    pub price_impact: Decimal,
    /// 执行分割路由的总 gas 成本
    pub gas_estimate: Decimal,
}

/// 路由请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    /// 输入代币符号
    pub input_token: String,
    /// 输出代币符号
    pub output_token: String,
    /// 输入代币数量
    pub input_amount: Decimal,
    /// 滑点容差（0-1 之间的小数）
    pub slippage_tolerance: Decimal,
    /// Bellman-Ford 算法的最大迭代次数
    pub max_iterations: usize,
    /// 是否启用分割路由功能
    pub enable_split_routes: bool,
    /// 分割路由的最大分割数量
    pub max_splits: Option<usize>,
}

/// 包含最优路径的路由响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    /// 原始路由请求
    pub request: RouteRequest,
    /// 找到的单个最优路由（如果找到）
    pub route: Option<Route>,
    /// 找到的分割路由（如果启用且找到）
    pub split_route: Option<SplitRoute>,
    /// 路由查找的执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 实际使用的迭代次数
    pub iterations_used: usize,
}

/// 路由图中的节点，带距离跟踪
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// 该节点对应的代币
    pub token: Token,
    /// Bellman-Ford 算法中的距离值
    pub distance: f64,
    /// 前驱节点的代币地址（用于路径重建）
    pub predecessor: Option<String>,
    /// 到达该节点时的最优代币数量
    pub best_amount: Decimal,
    /// 该节点已使用的流动性
    pub liquidity_used: Decimal,
}

/// Bellman-Ford 迭代状态
#[derive(Debug, Clone)]
pub struct IterationState {
    /// 图中所有节点的当前状态
    pub nodes: HashMap<String, GraphNode>,
    /// 当前迭代是否有改进
    pub improved: bool,
    /// 当前迭代次数
    pub iteration: usize,
    /// 目前找到的最优路由
    pub best_route: Option<Route>,
}

/// 获取实时价格的报价请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    /// 输入代币符号
    pub input_token: String,
    /// 输出代币符号
    pub output_token: String,
    /// 交易数量
    pub amount: Decimal,
    /// 请求报价的 DEX 平台名称
    pub dex_platform: String,
}

/// 包含价格信息的报价响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    /// 输入代币数量
    pub input_amount: Decimal,
    /// 输出代币数量
    pub output_amount: Decimal,
    /// 当前汇率
    pub exchange_rate: Decimal,
    /// 价格影响（滑点）
    pub price_impact: Decimal,
    /// 可用流动性
    pub liquidity_available: Decimal,
    /// 交易费用
    pub fee_amount: Decimal,
}

/// Metis 路由器的配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Bellman-Ford 算法的最大迭代次数
    pub max_iterations: usize,
    /// 最小流动性阈值，低于此值的边将被忽略
    pub min_liquidity_threshold: Decimal,
    /// 最大价格影响阈值，超过此值的路由将被拒绝
    pub max_price_impact: Decimal,
    /// 每笔交易的 gas 价格（以 SOL 为单位）
    pub gas_price: Decimal,
    /// 是否启用缓存功能
    pub enable_caching: bool,
    /// 缓存条目的生存时间（秒）
    pub cache_ttl_seconds: u64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            min_liquidity_threshold: dec!(100.0),
            max_price_impact: dec!(0.05), // 5%
            gas_price: dec!(0.000005), // 每笔交易的 SOL
            enable_caching: true,
            cache_ttl_seconds: 30,
        }
    }
} 