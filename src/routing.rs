use crate::graph::RoutingGraph;
use crate::quote::QuoteService;
use crate::types::*;
use anyhow::Result;
use log::{info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Instant;

/// 协调路由算法的主要 Metis 路由器
pub struct MetisRouter {
    /// 路由图，包含所有代币和交易对信息
    graph: RoutingGraph,
    /// 报价服务，用于获取实时价格
    quote_service: QuoteService,
    /// 路由器配置参数
    config: RouterConfig,
}

impl MetisRouter {
    pub fn new() -> Self {
        let config = RouterConfig::default();
        let graph = RoutingGraph::new(config.clone());
        let quote_service = QuoteService::new();
        
        Self {
            graph,
            quote_service,
            config,
        }
    }

    /// 用示例数据初始化路由器（用于演示）
    pub fn initialize(&mut self) {
        info!("🚀 用示例数据初始化 Metis 路由器");
        self.graph.initialize_sample_data();
    }

    /// 寻找最优路由的主要入口点
    pub async fn find_optimal_route(&self, request: RouteRequest) -> Result<RouteResponse> {
        let start_time = Instant::now();
        
        info!("🎯 处理路由请求: {} -> {} ({} {})", 
              request.input_token, request.output_token, 
              request.input_amount, request.input_token);

        // 验证请求
        self.validate_request(&request)?;

        let mut response = RouteResponse {
            request: request.clone(),
            route: None,
            split_route: None,
            execution_time_ms: 0,
            iterations_used: 0,
        };

        // 首先尝试找到单个最优路由
        if let Some(route) = self.graph.find_optimal_route(&request).await? {
            response.route = Some(route);
            info!("✅ 找到单个最优路由");
        } else {
            warn!("⚠️  未找到单个路由，尝试分割路由");
        }

        // 如果启用了分割路由且没有找到单个路由，尝试分割路由
        if request.enable_split_routes && response.route.is_none() {
            if let Some(split_route) = self.graph.find_split_routes(&request).await? {
                response.split_route = Some(split_route);
                info!("✅ 找到分割路由配置");
            }
        }

        // 如果我们同时有单个和分割路由，比较它们
        if let (Some(single_route), Some(split_route)) = (&response.route, &response.split_route) {
            let single_better = self.compare_routes(single_route, &split_route);
            if !single_better {
                info!("🔄 分割路由更好，移除单个路由");
                response.route = None;
            } else {
                info!("🔄 单个路由更好，移除分割路由");
                response.split_route = None;
            }
        }

        response.execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        if response.route.is_some() || response.split_route.is_some() {
            info!("✅ 路由查找在 {}ms 内成功完成", response.execution_time_ms);
        } else {
            warn!("❌ 未找到有效路由");
        }

        Ok(response)
    }

    /// 验证路由请求
    fn validate_request(&self, request: &RouteRequest) -> Result<()> {
        if request.input_amount <= dec!(0) {
            return Err(anyhow::anyhow!("输入数量必须为正数"));
        }

        if request.slippage_tolerance <= dec!(0) || request.slippage_tolerance >= dec!(1) {
            return Err(anyhow::anyhow!("滑点容差必须在 0 和 1 之间"));
        }

        if request.max_iterations == 0 {
            return Err(anyhow::anyhow!("最大迭代次数必须大于 0"));
        }

        if request.input_token == request.output_token {
            return Err(anyhow::anyhow!("输入和输出代币必须不同"));
        }

        Ok(())
    }

    /// 比较单个路由与分割路由以确定哪个更好
    fn compare_routes(&self, single_route: &Route, split_route: &SplitRoute) -> bool {
        // 考虑 gas 成本计算有效汇率
        let single_effective = single_route.effective_rate - single_route.gas_estimate;
        let split_effective = split_route.effective_rate - split_route.gas_estimate;
        
        // 如果有效汇率更高，单个路由更好
        single_effective > split_effective
    }

    /// 获取特定交易对的实时报价
    pub async fn get_quote(&self, request: &QuoteRequest) -> Result<QuoteResponse> {
        self.quote_service.get_quote(request).await
    }

    /// 用新鲜市场数据更新路由图
    pub async fn update_market_data(&mut self) -> Result<()> {
        info!("📊 更新路由图的市场数据");
        
        // 在实际实现中，这将：
        // 1. 从多个 DEX API 获取当前价格
        // 2. 更新流动性信息
        // 3. 刷新汇率
        // 4. 更新边权重
        
        // 为了演示，我们只记录更新
        info!("✅ 市场数据更新成功");
        Ok(())
    }

    /// 分析路由性能并提供见解
    pub fn analyze_route(&self, route: &Route) -> RouteAnalysis {
        let mut analysis = RouteAnalysis {
            total_hops: route.segments.len(),
            avg_price_impact: dec!(0),
            total_fees: dec!(0),
            efficiency_score: 0.0,
            recommendations: Vec::new(),
        };

        if !route.segments.is_empty() {
            // 计算平均价格影响
            analysis.avg_price_impact = route.price_impact / Decimal::from(route.segments.len());
            
            // 计算总费用
            analysis.total_fees = route.segments.iter()
                .map(|s| s.input_amount * s.dex_platform.fee_rate)
                .sum();
            
            // 计算效率分数（越高越好）
            let base_score = 1.0 - route.price_impact.to_string().parse::<f64>().unwrap_or(0.0);
            let fee_penalty = analysis.total_fees.to_string().parse::<f64>().unwrap_or(0.0) * 10.0;
            let hop_penalty = route.segments.len() as f64 * 0.1;
            analysis.efficiency_score = (base_score - fee_penalty - hop_penalty).max(0.0);
            
            // 生成建议
            if route.price_impact > dec!(0.02) {
                analysis.recommendations.push("考虑分割交易以减少价格影响".to_string());
            }
            
            if route.segments.len() > 2 {
                analysis.recommendations.push("路由有很多跳数，考虑直接交易对".to_string());
            }
            
            if analysis.total_fees > dec!(10) {
                analysis.recommendations.push("检测到高费用，考虑替代 DEX".to_string());
            }
        }

        analysis
    }

    /// 获取路由统计和性能指标
    pub fn get_routing_stats(&self) -> RoutingStats {
        RoutingStats {
            total_nodes: self.graph.nodes.len(),
            total_edges: self.graph.edges.values().map(|v| v.len()).sum(),
            cache_hit_rate: 0.85, // 示例值
            avg_execution_time_ms: 45, // 示例值
            success_rate: 0.92, // 示例值
        }
    }
}

/// 路由性能分析
#[derive(Debug, Clone)]
pub struct RouteAnalysis {
    /// 路由的总跳数（路径段数量）
    pub total_hops: usize,
    /// 平均价格影响（每个跳转的平均滑点）
    pub avg_price_impact: Decimal,
    /// 总交易费用
    pub total_fees: Decimal,
    /// 效率分数（0-1 之间，越高越好）
    pub efficiency_score: f64,
    /// 改进建议列表
    pub recommendations: Vec<String>,
}

/// 路由统计和性能指标
#[derive(Debug, Clone)]
pub struct RoutingStats {
    /// 路由图中的总节点数（代币数量）
    pub total_nodes: usize,
    /// 路由图中的总边数（交易对数量）
    pub total_edges: usize,
    /// 缓存命中率（0-1 之间的小数）
    pub cache_hit_rate: f64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: u64,
    /// 路由查找成功率（0-1 之间的小数）
    pub success_rate: f64,
}

impl Default for MetisRouter {
    fn default() -> Self {
        Self::new()
    }
} 