use crate::types::*;
use anyhow::Result;
use dashmap::DashMap;
use log::{debug, info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 处理来自 DEX 平台的实时报价服务
pub struct QuoteService {
    /// 报价缓存，键为缓存键，值为带过期时间的缓存报价
    cache: Arc<DashMap<String, CachedQuote>>,
    /// 报价服务配置参数
    config: QuoteConfig,
}

/// 报价服务配置
#[derive(Debug, Clone)]
pub struct QuoteConfig {
    /// 缓存条目的生存时间（秒）
    pub cache_ttl_seconds: u64,
    /// 获取报价失败时的最大重试次数
    pub max_retries: u32,
    /// 请求超时时间（秒）
    pub timeout_seconds: u64,
    /// 是否启用缓存功能
    pub enable_cache: bool,
}

impl Default for QuoteConfig {
    fn default() -> Self {
        Self {
            cache_ttl_seconds: 30,
            max_retries: 3,
            timeout_seconds: 10,
            enable_cache: true,
        }
    }
}

/// 带过期时间的缓存报价
#[derive(Debug, Clone)]
struct CachedQuote {
    /// 缓存的报价响应
    quote: QuoteResponse,
    /// 缓存条目的过期时间
    expires_at: Instant,
}

impl QuoteService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            config: QuoteConfig::default(),
        }
    }

    /// 获取特定交易对的报价
    pub async fn get_quote(&self, request: &QuoteRequest) -> Result<QuoteResponse> {
        let cache_key = self.generate_cache_key(request);
        
        // 首先检查缓存
        if self.config.enable_cache {
            if let Some(cached) = self.cache.get(&cache_key) {
                if cached.expires_at > Instant::now() {
                    debug!("📋 报价缓存命中: {} -> {}", 
                           request.input_token, request.output_token);
                    return Ok(cached.quote.clone());
                } else {
                    // 移除过期的缓存条目
                    self.cache.remove(&cache_key);
                }
            }
        }

        // 获取新鲜报价
        let quote = self.fetch_quote_from_dex(request).await?;
        
        // 缓存结果
        if self.config.enable_cache {
            let cached_quote = CachedQuote {
                quote: quote.clone(),
                expires_at: Instant::now() + Duration::from_secs(self.config.cache_ttl_seconds),
            };
            self.cache.insert(cache_key, cached_quote);
        }

        Ok(quote)
    }

    /// 从 DEX 平台获取报价（模拟演示）
    async fn fetch_quote_from_dex(&self, request: &QuoteRequest) -> Result<QuoteResponse> {
        info!("🔍 从 {} 获取报价: {} {} -> {}", 
              request.dex_platform, request.amount, 
              request.input_token, request.output_token);

        // 模拟 API 调用延迟
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 根据平台模拟不同的 DEX 响应
        let (exchange_rate, liquidity, fee_rate) = match request.dex_platform.as_str() {
            "Raydium" => (dec!(0.001), dec!(1000000), dec!(0.0025)),
            "Orca" => (dec!(0.00101), dec!(500000), dec!(0.003)),
            "Meteora" => (dec!(0.00102), dec!(2000000), dec!(0.0035)),
            _ => (dec!(0.001), dec!(100000), dec!(0.003)),
        };

        // 计算带价格影响的输出数量
        let base_output = request.amount * exchange_rate;
        let price_impact = self.calculate_price_impact(request.amount, liquidity);
        let output_amount = base_output * (dec!(1) - price_impact);
        
        // 计算费用
        let fee_amount = request.amount * fee_rate;

        let quote = QuoteResponse {
            input_amount: request.amount,
            output_amount,
            exchange_rate,
            price_impact,
            liquidity_available: liquidity,
            fee_amount,
        };

        debug!("✅ 收到报价: {} {} -> {} {} (汇率: {}, 影响: {})", 
               request.amount, request.input_token, 
               output_amount, request.output_token,
               exchange_rate, price_impact);

        Ok(quote)
    }

    /// 根据交易规模和流动性计算价格影响
    fn calculate_price_impact(&self, trade_amount: Decimal, liquidity: Decimal) -> Decimal {
        // 简单的线性价格影响模型
        // 实际应用中，这将使用实际的 DEX 曲线公式
        let impact_ratio = trade_amount / liquidity;
        impact_ratio * dec!(0.5) // 比率的 50% 作为价格影响
    }

    /// 为报价请求生成缓存键
    fn generate_cache_key(&self, request: &QuoteRequest) -> String {
        format!("{}:{}:{}:{}", 
                request.dex_platform, 
                request.input_token, 
                request.output_token, 
                request.amount)
    }

    /// 并行获取多个 DEX 平台的报价
    pub async fn get_multi_dex_quotes(
        &self,
        input_token: &str,
        output_token: &str,
        amount: Decimal,
        dex_platforms: &[String],
    ) -> Result<Vec<(String, QuoteResponse)>> {
        let mut quote_futures = Vec::new();

        for dex in dex_platforms {
            let request = QuoteRequest {
                input_token: input_token.to_string(),
                output_token: output_token.to_string(),
                amount,
                dex_platform: dex.clone(),
            };
            
            let quote_service = self.clone();
            let future = async move {
                match quote_service.get_quote(&request).await {
                    Ok(quote) => Some((dex.clone(), quote)),
                    Err(e) => {
                        warn!("❌ 从 {} 获取报价失败: {}", dex, e);
                        None
                    }
                }
            };
            
            quote_futures.push(future);
        }

        // 并行执行所有报价请求
        let results = futures::future::join_all(quote_futures).await;
        
        // 过滤掉失败的请求
        let quotes: Vec<(String, QuoteResponse)> = results
            .into_iter()
            .filter_map(|result| result)
            .collect();

        info!("📊 从 {} 个 DEX 平台获取了 {} 个报价", 
              dex_platforms.len(), quotes.len());

        Ok(quotes)
    }

    /// 清理过期的缓存条目
    pub fn cleanup_cache(&self) {
        let now = Instant::now();
        let mut expired_keys = Vec::new();

        for entry in self.cache.iter() {
            if entry.expires_at <= now {
                expired_keys.push(entry.key().clone());
            }
        }

        let removed_count = expired_keys.len();
        for key in expired_keys {
            self.cache.remove(&key);
        }

        debug!("🧹 清理了 {} 个过期缓存条目", removed_count);
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> CacheStats {
        let total_entries = self.cache.len();
        let now = Instant::now();
        let expired_entries = self.cache.iter()
            .filter(|entry| entry.expires_at <= now)
            .count();

        CacheStats {
            total_entries,
            expired_entries,
            valid_entries: total_entries - expired_entries,
            cache_hit_rate: 0.85, // 这将从实际使用中计算
        }
    }
}

impl Clone for QuoteService {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
            config: self.config.clone(),
        }
    }
}

/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 缓存中的总条目数
    pub total_entries: usize,
    /// 已过期的条目数
    pub expired_entries: usize,
    /// 有效的条目数
    pub valid_entries: usize,
    /// 缓存命中率（0-1 之间的小数）
    pub cache_hit_rate: f64,
}

impl Default for QuoteService {
    fn default() -> Self {
        Self::new()
    }
} 