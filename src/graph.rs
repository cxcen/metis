use crate::types::*;
use anyhow::Result;
use dashmap::DashMap;
use log::{debug, info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Metis 路由算法的图表示
pub struct RoutingGraph {
    pub nodes: HashMap<String, Token>,
    pub edges: HashMap<String, Vec<Edge>>, // token_address -> edges
    pub config: RouterConfig,
    pub quote_cache: Arc<DashMap<String, QuoteResponse>>,
}

impl RoutingGraph {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            config,
            quote_cache: Arc::new(DashMap::new()),
        }
    }

    /// 向图中添加代币
    pub fn add_token(&mut self, token: Token) {
        self.nodes.insert(token.address.clone(), token);
    }

    /// 向图中添加边（交易对）
    pub fn add_edge(&mut self, edge: Edge) {
        let from_addr = edge.from_token.address.clone();
        self.edges
            .entry(from_addr)
            .or_insert_with(Vec::new)
            .push(edge);
    }

    /// 用示例数据初始化图（用于演示）
    pub fn initialize_sample_data(&mut self) {
        // 添加示例代币
        let usdc = Token {
            symbol: "USDC".to_string(),
            address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            decimals: 6,
        };
        let sol = Token {
            symbol: "SOL".to_string(),
            address: "So11111111111111111111111111111111111111112".to_string(),
            decimals: 9,
        };
        let ray = Token {
            symbol: "RAY".to_string(),
            address: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R".to_string(),
            decimals: 6,
        };

        self.add_token(usdc.clone());
        self.add_token(sol.clone());
        self.add_token(ray.clone());

        // 添加不同 DEX 平台的示例边
        let raydium = DexPlatform {
            name: "Raydium".to_string(),
            address: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
            fee_rate: dec!(0.0025), // 0.25%
        };

        let orca = DexPlatform {
            name: "Orca".to_string(),
            address: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(),
            fee_rate: dec!(0.003), // 0.3%
        };

        let meteora = DexPlatform {
            name: "Meteora".to_string(),
            address: "MeteoraDLK6sc2NfSy2vM6iBzf6Vwj2MmZ1T3YDV4whf".to_string(),
            fee_rate: dec!(0.0035), // 0.35%
        };

        // USDC -> SOL 边
        self.add_edge(Edge {
            from_token: usdc.clone(),
            to_token: sol.clone(),
            dex_platform: raydium.clone(),
            exchange_rate: dec!(0.001),   // 1 SOL = 1000 USDC
            liquidity: dec!(1000000),     // 100万 USDC 流动性
            max_trade_size: dec!(500000), // 50万 USDC 最大交易
            min_trade_size: dec!(10),     // 10 USDC 最小交易
            weight: -f64::ln(0.001),      // -log(exchange_rate)
        });

        self.add_edge(Edge {
            from_token: usdc.clone(),
            to_token: sol.clone(),
            dex_platform: orca.clone(),
            exchange_rate: dec!(0.00101), // 稍差的汇率
            liquidity: dec!(500000),      // 50万 USDC 流动性
            max_trade_size: dec!(200000), // 20万 USDC 最大交易
            min_trade_size: dec!(10),     // 10 USDC 最小交易
            weight: -f64::ln(0.00101),
        });

        self.add_edge(Edge {
            from_token: usdc.clone(),
            to_token: sol.clone(),
            dex_platform: meteora.clone(),
            exchange_rate: dec!(0.00102),  // 最差汇率但流动性好
            liquidity: dec!(2000000),      // 200万 USDC 流动性
            max_trade_size: dec!(1000000), // 100万 USDC 最大交易
            min_trade_size: dec!(10),      // 10 USDC 最小交易
            weight: -f64::ln(0.00102),
        });

        // USDC -> RAY 边
        self.add_edge(Edge {
            from_token: usdc.clone(),
            to_token: ray.clone(),
            dex_platform: raydium.clone(),
            exchange_rate: dec!(0.5),    // 1 RAY = 0.5 USDC
            liquidity: dec!(100000),     // 10万 USDC 流动性
            max_trade_size: dec!(50000), // 5万 USDC 最大交易
            min_trade_size: dec!(10),    // 10 USDC 最小交易
            weight: -f64::ln(0.5),
        });

        // RAY -> SOL 边
        self.add_edge(Edge {
            from_token: ray.clone(),
            to_token: sol.clone(),
            dex_platform: orca.clone(),
            exchange_rate: dec!(0.002),  // 1 SOL = 500 RAY
            liquidity: dec!(50000),      // 5万 RAY 流动性
            max_trade_size: dec!(25000), // 2.5万 RAY 最大交易
            min_trade_size: dec!(1),     // 1 RAY 最小交易
            weight: -f64::ln(0.002),
        });
    }

    /// 具有 Metis 改进的增强 Bellman-Ford 算法
    pub async fn find_optimal_route(&self, request: &RouteRequest) -> Result<Option<Route>> {
        let start_time = std::time::Instant::now();

        info!(
            "🔍 寻找最优路由: {} -> {} ({} {})",
            request.input_token, request.output_token, request.input_amount, request.input_token
        );

        // 初始化节点
        let mut nodes = self.initialize_nodes(&request.input_token)?;

        // 设置起始节点
        let start_addr = self.get_token_address(&request.input_token)?;
        if let Some(start_node) = nodes.get_mut(&start_addr) {
            start_node.distance = 0.0;
            start_node.best_amount = request.input_amount;
        }

        let mut iteration_state = IterationState {
            nodes,
            improved: true,
            iteration: 0,
            best_route: None,
        };

        // 具有早期终止的增强 Bellman-Ford 迭代
        while iteration_state.improved && iteration_state.iteration < request.max_iterations {
            iteration_state.improved = false;
            iteration_state.iteration += 1;

            debug!("🔄 Bellman-Ford 迭代 {}", iteration_state.iteration);

            // 处理具有流动性约束的所有边
            for (from_addr, edges) in &self.edges {
                if let Some(_from_node) = iteration_state.nodes.get(from_addr) {
                    for edge in edges {
                        self.relax_edge(&mut iteration_state, edge, request).await?;
                    }
                }
            }

            // 如果没有改进则早期终止
            if !iteration_state.improved {
                debug!("✅ 迭代 {} 中没有改进，提前终止", iteration_state.iteration);
                break;
            }
        }

        // 提取找到的最优路由
        let route = self.extract_route(&iteration_state, request)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        info!(
            "⏱️  路由查找在 {}ms 内完成 ({} 次迭代)",
            execution_time, iteration_state.iteration
        );

        Ok(route)
    }

    /// 为 Bellman-Ford 初始化图节点
    fn initialize_nodes(&self, _start_token: &str) -> Result<HashMap<String, GraphNode>> {
        let mut nodes = HashMap::new();

        for (addr, token) in &self.nodes {
            nodes.insert(
                addr.clone(),
                GraphNode {
                    token: token.clone(),
                    distance: f64::INFINITY,
                    predecessor: None,
                    best_amount: dec!(0),
                    liquidity_used: dec!(0),
                },
            );
        }

        Ok(nodes)
    }

    /// 具有流动性约束的增强松弛操作
    async fn relax_edge(
        &self,
        state: &mut IterationState,
        edge: &Edge,
        _request: &RouteRequest,
    ) -> Result<()> {
        let from_addr = &edge.from_token.address;
        let to_addr = &edge.to_token.address;

        if let Some(from_node) = state.nodes.get(from_addr) {
            if from_node.distance == f64::INFINITY {
                return Ok(()); // 跳过不可达节点
            }

            // 计算潜在改进
            let new_distance = from_node.distance + edge.weight;
            let potential_amount = from_node.best_amount * edge.exchange_rate;

            // 应用流动性约束
            let available_liquidity = edge.liquidity - edge.max_trade_size.min(edge.liquidity);
            let constrained_amount = potential_amount.min(available_liquidity);

            // 检查此路径是否更好
            if let Some(to_node) = state.nodes.get_mut(to_addr) {
                if new_distance < to_node.distance && constrained_amount > dec!(0) {
                    // 额外约束：价格影响、最小交易规模
                    if constrained_amount >= edge.min_trade_size
                        && self.calculate_price_impact(edge, constrained_amount)
                            <= self.config.max_price_impact
                    {
                        to_node.distance = new_distance;
                        to_node.predecessor = Some(from_addr.clone());
                        to_node.best_amount = constrained_amount;
                        to_node.liquidity_used = constrained_amount;

                        state.improved = true;

                        debug!(
                            "🔄 松弛边: {} -> {} (数量: {}, 距离: {})",
                            edge.from_token.symbol,
                            edge.to_token.symbol,
                            constrained_amount,
                            new_distance
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// 计算给定交易规模的价格影响
    fn calculate_price_impact(&self, edge: &Edge, trade_amount: Decimal) -> Decimal {
        // 简单的线性价格影响模型
        // 实际应用中，这将使用实际的 DEX 曲线（恒定乘积等）
        let impact_ratio = trade_amount / edge.liquidity;
        impact_ratio * dec!(0.5) // 比率的 50% 作为价格影响
    }

    /// 从 Bellman-Ford 结果中提取最优路由
    fn extract_route(
        &self,
        state: &IterationState,
        request: &RouteRequest,
    ) -> Result<Option<Route>> {
        let output_addr = self.get_token_address(&request.output_token)?;

        if let Some(output_node) = state.nodes.get(&output_addr) {
            if output_node.distance == f64::INFINITY {
                warn!("❌ 未找到到输出代币 {} 的路径", request.output_token);
                return Ok(None);
            }

            // 重建路径
            let mut segments = Vec::new();
            let mut current_addr = output_addr.clone();
            let mut current_amount = output_node.best_amount;

            while let Some(predecessor_addr) = &state.nodes[&current_addr].predecessor {
                let edge = self.find_edge(predecessor_addr, &current_addr)?;
                let predecessor_node = &state.nodes[predecessor_addr];

                let input_amount = predecessor_node.best_amount;
                let output_amount = current_amount;
                let exchange_rate = output_amount / input_amount;
                let price_impact = self.calculate_price_impact(edge, input_amount);

                segments.push(PathSegment {
                    from_token: edge.from_token.clone(),
                    to_token: edge.to_token.clone(),
                    dex_platform: edge.dex_platform.clone(),
                    input_amount,
                    output_amount,
                    exchange_rate,
                    price_impact,
                });

                current_addr = predecessor_addr.clone();
                current_amount = input_amount;
            }

            // 反转段以获得正确顺序
            segments.reverse();

            if segments.is_empty() {
                return Ok(None);
            }

            let total_input = request.input_amount;
            let total_output = segments.last().unwrap().output_amount;
            let effective_rate = total_output / total_input;
            let total_price_impact = segments.iter().map(|s| s.price_impact).sum();
            let gas_estimate = self.estimate_gas_cost(&segments);

            Ok(Some(Route {
                segments,
                total_input_amount: total_input,
                total_output_amount: total_output,
                effective_rate,
                price_impact: total_price_impact,
                gas_estimate,
                split_ratio: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// 查找两个代币之间的边
    fn find_edge(&self, from_addr: &str, to_addr: &str) -> Result<&Edge> {
        if let Some(edges) = self.edges.get(from_addr) {
            for edge in edges {
                if edge.to_token.address == to_addr {
                    return Ok(edge);
                }
            }
        }
        Err(anyhow::anyhow!("未找到边: {} -> {}", from_addr, to_addr))
    }

    /// 通过符号获取代币地址
    fn get_token_address(&self, symbol: &str) -> Result<String> {
        for (addr, token) in &self.nodes {
            if token.symbol == symbol {
                return Ok(addr.clone());
            }
        }
        Err(anyhow::anyhow!("未找到代币: {}", symbol))
    }

    /// 通过符号获取代币
    fn get_token_by_symbol(&self, symbol: &str) -> Result<&Token> {
        for (_, token) in &self.nodes {
            if token.symbol == symbol {
                return Ok(token);
            }
        }
        Err(anyhow::anyhow!("未找到代币: {}", symbol))
    }

    /// 估算路由的 gas 成本
    fn estimate_gas_cost(&self, segments: &[PathSegment]) -> Decimal {
        // 简单的 gas 估算：每个 DEX 交互的基础成本
        let base_gas_per_dex = dec!(0.000001); // 每次 DEX 交互的 SOL
        let total_gas = base_gas_per_dex * Decimal::from(segments.len());
        total_gas * self.config.gas_price
    }

    /// 寻找分割路由以获得更好的执行
    pub async fn find_split_routes(&self, request: &RouteRequest) -> Result<Option<SplitRoute>> {
        if !request.enable_split_routes {
            return Ok(None);
        }

        info!(
            "🔀 为 {} {} 寻找分割路由",
            request.input_amount, request.input_token
        );

        let mut split_routes = Vec::new();
        let mut remaining_amount = request.input_amount;
        let max_splits = request.max_splits.unwrap_or(3);

        // 尝试找到具有不同数量的多个路由
        for split_idx in 0..max_splits {
            if remaining_amount <= dec!(0) {
                break;
            }

            // 计算分割数量（递减部分）
            let split_ratio = if split_idx == 0 {
                dec!(0.6)
            } else if split_idx == 1 {
                dec!(0.3)
            } else {
                dec!(0.1)
            };

            let split_amount = request.input_amount * split_ratio;

            if split_amount < dec!(10) {
                // 最小可行数量
                break;
            }

            let mut split_request = request.clone();
            split_request.input_amount = split_amount;

            if let Some(route) = self.find_optimal_route(&split_request).await? {
                split_routes.push(route);
                remaining_amount -= split_amount;
            }
        }

        if split_routes.is_empty() {
            return Ok(None);
        }

        // 计算组合指标
        let total_input = split_routes.iter().map(|r| r.total_input_amount).sum();
        let total_output = split_routes.iter().map(|r| r.total_output_amount).sum();
        let effective_rate = total_output / total_input;
        let total_price_impact = split_routes.iter().map(|r| r.price_impact).sum();
        let total_gas = split_routes.iter().map(|r| r.gas_estimate).sum();

        Ok(Some(SplitRoute {
            routes: split_routes,
            total_input_amount: total_input,
            total_output_amount: total_output,
            effective_rate,
            price_impact: total_price_impact,
            gas_estimate: total_gas,
        }))
    }
}
