use reqwest::{Client, header::HeaderMap};
use scraper::{Html, Selector};
use std::time::Duration;
use crate::error::{AppError, AppResult};
use headless_chrome::{Browser, LaunchOptions};

pub struct orbScraper {
    client: Client,
    balance_selector: Selector,
    retry_attempts: u32,
    timeout_seconds: u64,
    use_browser: bool,
}

impl orbScraper {
    pub async fn new() -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()?;

        // CSS selector to find elements (we'll search text manually)
        let balance_selector = Selector::parse("*")
            .map_err(|e| AppError::Scraping(format!("Invalid CSS selector: {}", e)))?;

        Ok(Self {
            client,
            balance_selector,
            retry_attempts: 3,
            timeout_seconds: 30,
            use_browser: true, // Enable browser-based scraping for JavaScript content
        })
    }
    
    pub async fn fetch_balance(&self, token: &str) -> AppResult<u32> {
        let url = format!("https://portal.withorb.com/view?token={}", token);

        for attempt in 1..=self.retry_attempts {
            // First try direct API approach
            tracing::info!("Attempt {}: Trying direct API approach first...", attempt);
            match self.try_direct_api_approach(&url).await {
                Ok(balance) => {
                    tracing::info!("✅ Successfully fetched balance via API: {} (attempt {})", balance, attempt);
                    return Ok(balance);
                }
                Err(e) => {
                    tracing::info!("API approach failed: {}, trying browser...", e);
                }
            }

            let result = if self.use_browser {
                // Use headless browser for JavaScript-rendered content
                self.try_fetch_balance_with_browser(&url).await
            } else {
                // Fallback to HTTP scraping
                self.try_fetch_balance_enhanced(&url).await
            };

            match result {
                Ok(balance) => {
                    tracing::info!("Successfully fetched balance: {} (attempt {})", balance, attempt);
                    return Ok(balance);
                }
                Err(e) => {
                    tracing::warn!("Attempt {} failed: {}", attempt, e);
                    if attempt == self.retry_attempts {
                        return Err(e);
                    }

                    // Exponential backoff
                    let delay = Duration::from_secs(2_u64.pow(attempt - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(AppError::Scraping("All retry attempts failed".to_string()))
    }

    async fn try_fetch_balance_with_browser(&self, url: &str) -> AppResult<u32> {
        tracing::info!("Using enhanced browser simulation to extract balance from: {}", url);

        // Launch headless Chrome with realistic settings
        let launch_options = LaunchOptions::default_builder()
            .headless(true)
            .sandbox(false)
            .window_size(Some((1920, 1080)))

            .build()
            .map_err(|e| AppError::Scraping(format!("Failed to build launch options: {}", e)))?;

        let browser = Browser::new(launch_options)
            .map_err(|e| AppError::Scraping(format!("Failed to launch browser: {}", e)))?;

        let tab = browser.new_tab()
            .map_err(|e| AppError::Scraping(format!("Failed to create new tab: {}", e)))?;

        // Set up realistic browser environment
        tracing::info!("Setting up realistic browser environment...");

        // Set realistic user agent and properties
        let stealth_script = r#"
            // Override webdriver property
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined,
            });

            // Override plugins to look like a real browser
            Object.defineProperty(navigator, 'plugins', {
                get: () => [
                    { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer' },
                    { name: 'Chrome PDF Viewer', filename: 'mhjfbmdgcfjbbpaeojofohoefgiehjai' },
                    { name: 'Native Client', filename: 'internal-nacl-plugin' }
                ],
            });

            // Override languages
            Object.defineProperty(navigator, 'languages', {
                get: () => ['en-US', 'en'],
            });

            // Add chrome runtime
            window.chrome = {
                runtime: {},
                loadTimes: function() { return {}; },
                csi: function() { return {}; }
            };

            // Override permissions
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = (parameters) => (
                parameters.name === 'notifications' ?
                    Promise.resolve({ state: Notification.permission }) :
                    originalQuery(parameters)
            );
        "#;

        if let Err(e) = tab.evaluate(stealth_script, false) {
            tracing::warn!("Failed to set stealth properties: {}", e);
        }

        // Navigate to the orb portal
        tracing::info!("Navigating to orb portal...");
        tab.navigate_to(url)
            .map_err(|e| AppError::Scraping(format!("Failed to navigate to URL: {}", e)))?;

        // Wait for the page to load and simulate real user behavior
        tracing::info!("Waiting for page to load and simulating user interaction...");

        // Wait for initial page load
        std::thread::sleep(Duration::from_secs(3));

        // Simulate user behavior: scroll, move mouse, wait
        tracing::info!("Simulating user interaction to trigger content loading...");

        // Scroll to trigger any lazy loading
        if let Err(e) = tab.evaluate("window.scrollTo(0, 100);", false) {
            tracing::warn!("Failed to scroll: {}", e);
        }
        std::thread::sleep(Duration::from_secs(1));

        // Scroll back to top
        if let Err(e) = tab.evaluate("window.scrollTo(0, 0);", false) {
            tracing::warn!("Failed to scroll to top: {}", e);
        }
        std::thread::sleep(Duration::from_secs(2));

        // Try to trigger any click events that might load data
        if let Err(e) = tab.evaluate("document.body.click();", false) {
            tracing::warn!("Failed to click body: {}", e);
        }
        std::thread::sleep(Duration::from_secs(2));

        // Check if content has loaded by monitoring page changes and looking for specific content
        let mut previous_content_length = 0;
        let mut balance_found = false;

        for attempt in 1..=10 { // Increased attempts
            let current_content = tab.get_content()
                .map_err(|e| AppError::Scraping(format!("Failed to get page content during wait: {}", e)))?;

            tracing::info!("Wait attempt {}: Content length = {}", attempt, current_content.len());

            // Look for the specific balance text pattern
            if current_content.contains("Credit balance") ||
               current_content.contains("User Messages") ||
               current_content.contains("2,666") {
                tracing::info!("✅ Found balance content! Breaking early.");
                balance_found = true;
                break;
            }

            // If content has changed significantly, it might have loaded
            if current_content.len() > previous_content_length + 1000 {
                tracing::info!("Content appears to have loaded (length increased significantly)");
                // Don't break immediately, keep looking for balance content
            }

            // Log a sample of the content to see what we're getting
            if attempt <= 3 {
                let sample = if current_content.len() > 500 {
                    &current_content[..500]
                } else {
                    &current_content
                };
                tracing::debug!("Content sample (attempt {}): {}", attempt, sample);
            }

            previous_content_length = current_content.len();
            std::thread::sleep(Duration::from_secs(3)); // Increased wait time
        }

        if balance_found {
            tracing::info!("Balance content detected, proceeding with extraction");
        } else {
            tracing::warn!("Balance content not detected after waiting, proceeding anyway");
        }

        // Try to trigger any lazy loading by scrolling and executing JavaScript
        tracing::info!("Attempting to trigger content loading...");
        if let Err(e) = tab.evaluate("window.scrollTo(0, document.body.scrollHeight);", false) {
            tracing::warn!("Failed to scroll page: {}", e);
        }

        // Wait a bit more after scrolling
        std::thread::sleep(Duration::from_secs(2));

        // Wait for potential balance elements to appear
        let balance_selectors = [
            "span[data-testid='balance']",
            ".balance",
            ".credit-balance",
            "[class*='balance']",
            "[class*='credit']",
            "[data-balance]",
            "[data-credits]",
            "span:contains('$')",
            "div:contains('balance')",
            "span:contains('credit')",
            "div:contains('$')",
            "*:contains('remaining')",
            "*:contains('available')",
        ];

        // Try to find balance using different selectors
        for selector in &balance_selectors {
            tracing::debug!("Trying selector: {}", selector);
            if let Ok(element) = tab.wait_for_element_with_custom_timeout(selector, Duration::from_secs(2)) {
                if let Ok(text) = element.get_inner_text() {
                    tracing::info!("Found element with selector '{}': '{}'", selector, text);
                    if let Some(balance) = self.extract_number_from_text(&text) {
                        tracing::info!("✅ Extracted balance from browser: {}", balance);
                        return Ok(balance);
                    }
                }
            }
        }

        // Try to extract balance from JavaScript variables
        tracing::info!("Attempting to extract balance from JavaScript variables...");
        let js_queries = [
            "window.__NEXT_DATA__?.props?.pageProps?.balance",
            "window.__NEXT_DATA__?.props?.pageProps?.credits",
            "window.__NEXT_DATA__?.props?.pageProps?.customer?.balance",
            "window.balance",
            "window.credits",
            "window.customerBalance",
            "document.querySelector('[data-balance]')?.textContent",
            "document.querySelector('[data-credits]')?.textContent",
            "Array.from(document.querySelectorAll('*')).find(el => el.textContent.includes('$'))?.textContent",
        ];

        for js_query in &js_queries {
            tracing::debug!("Trying JavaScript query: {}", js_query);
            if let Ok(result) = tab.evaluate(js_query, false) {
                if let Some(value) = result.value {
                    let value_str = format!("{:?}", value);
                    tracing::info!("JavaScript query '{}' returned: {}", js_query, value_str);

                    if let Some(balance) = self.extract_number_from_text(&value_str) {
                        tracing::info!("✅ Extracted balance from JavaScript: {}", balance);
                        return Ok(balance);
                    }
                }
            }
        }

        // If specific selectors don't work, get the full page content and search
        let html_content = tab.get_content()
            .map_err(|e| AppError::Scraping(format!("Failed to get page content: {}", e)))?;

        tracing::info!("Browser rendered page content length: {}", html_content.len());

        // Log a larger sample of the content to analyze what we're getting
        let content_sample = if html_content.len() > 2000 {
            &html_content[..2000]
        } else {
            &html_content
        };
        tracing::info!("Content sample for analysis: {}", content_sample);

        // Check if we can find the balance text directly in the HTML
        if html_content.contains("Credit balance") {
            tracing::info!("✅ Found 'Credit balance' text in HTML content!");
        } else {
            tracing::warn!("❌ 'Credit balance' text not found in HTML content");
        }

        if html_content.contains("User Messages") {
            tracing::info!("✅ Found 'User Messages' text in HTML content!");
        } else {
            tracing::warn!("❌ 'User Messages' text not found in HTML content");
        }

        // Try direct API approach based on common orb API patterns
        tracing::info!("Attempting direct API approach...");
        if let Ok(balance) = self.try_direct_api_approach(url).await {
            return Ok(balance);
        }

        // Parse the fully rendered HTML as fallback
        self.parse_balance_from_html(&html_content)
    }

    async fn try_direct_api_approach(&self, portal_url: &str) -> AppResult<u32> {
        // Extract token from portal URL
        let token = portal_url.split("token=").nth(1).unwrap_or("");

        tracing::info!("Attempting to fetch balance using orb Portal API endpoints with token: {}", token);

        // First, get customer information from the portal token
        let customer_info_url = format!("https://portal.withorb.com/api/v1/customer_from_link?token={}", token);

        // Set up headers to simulate browser request
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap());
        headers.insert("Referer", format!("https://portal.withorb.com/view?token={}", token).parse().unwrap());
        headers.insert("Origin", "https://portal.withorb.com".parse().unwrap());

        // Get customer information first
        tracing::info!("Fetching customer information from: {}", customer_info_url);
        let customer_response = self.client
            .get(&customer_info_url)
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| AppError::Scraping(format!("Failed to fetch customer info: {}", e)))?;

        if !customer_response.status().is_success() {
            return Err(AppError::Scraping(format!("Customer info API returned status: {}", customer_response.status())));
        }

        let customer_data: serde_json::Value = customer_response
            .json()
            .await
            .map_err(|e| AppError::Scraping(format!("Failed to parse customer info JSON: {}", e)))?;

        tracing::info!("Customer data received successfully");

        // Extract customer ID and pricing unit ID from the response
        let customer_id = customer_data
            .get("customer")
            .and_then(|c| c.get("id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| AppError::Scraping("Could not extract customer ID from response".to_string()))?;

        let pricing_unit_id = customer_data
            .get("customer")
            .and_then(|c| c.get("ledger_pricing_units"))
            .and_then(|units| units.as_array())
            .and_then(|arr| arr.first())
            .and_then(|unit| unit.get("id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| AppError::Scraping("Could not extract pricing unit ID from response".to_string()))?;

        tracing::info!("Extracted customer_id: {}, pricing_unit_id: {}", customer_id, pricing_unit_id);

        // Now fetch the ledger summary with the balance information
        let ledger_url = format!(
            "https://portal.withorb.com/api/v1/customers/{}/ledger_summary?pricing_unit_id={}&token={}",
            customer_id, pricing_unit_id, token
        );

        tracing::info!("Fetching ledger summary from: {}", ledger_url);
        let ledger_response = self.client
            .get(&ledger_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AppError::Scraping(format!("Failed to fetch ledger summary: {}", e)))?;

        if !ledger_response.status().is_success() {
            return Err(AppError::Scraping(format!("Ledger API returned status: {}", ledger_response.status())));
        }

        let ledger_data: serde_json::Value = ledger_response
            .json()
            .await
            .map_err(|e| AppError::Scraping(format!("Failed to parse ledger JSON: {}", e)))?;

        tracing::info!("Ledger data received successfully");

        // Extract the credits balance
        let credits_balance = ledger_data
            .get("credits_balance")
            .and_then(|balance| balance.as_str())
            .ok_or_else(|| AppError::Scraping("Could not extract credits_balance from ledger response".to_string()))?;

        // Parse the balance as a float and convert to u32
        let balance_float: f64 = credits_balance
            .parse()
            .map_err(|e| AppError::Scraping(format!("Failed to parse balance '{}' as number: {}", credits_balance, e)))?;

        let balance = balance_float as u32;
        tracing::info!("✅ Successfully extracted balance: {} credits", balance);

        Ok(balance)
    }

    fn extract_balance_from_json(&self, json: &serde_json::Value) -> Option<u32> {
        // Try different JSON paths where balance might be stored
        let paths = [
            "balance",
            "credit_balance",
            "credits",
            "remaining_balance",
            "customer.balance",
            "customer.credits",
            "data.balance",
            "data.credits",
            "result.balance",
        ];

        for path in &paths {
            if let Some(value) = self.get_json_value_by_path(json, path) {
                if let Some(balance) = self.extract_balance_from_json_value(value) {
                    return Some(balance);
                }
            }
        }

        None
    }

    fn get_json_value_by_path<'a>(&self, json: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    fn extract_balance_from_json_value(&self, value: &serde_json::Value) -> Option<u32> {
        match value {
            serde_json::Value::Number(n) => n.as_u64().map(|n| n as u32),
            serde_json::Value::String(s) => self.extract_number_from_text(s),
            _ => None,
        }
    }

    async fn try_fetch_balance_enhanced(&self, url: &str) -> AppResult<u32> {
        tracing::info!("Fetching balance from: {}", url);

        // Try multiple requests with different strategies
        for strategy in 1..=3 {
            match self.try_fetch_with_strategy(url, strategy).await {
                Ok(balance) => return Ok(balance),
                Err(e) => {
                    tracing::debug!("Strategy {} failed: {}", strategy, e);
                    if strategy < 3 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }

        Err(AppError::Scraping("Could not find credit balance with any strategy".to_string()))
    }

    async fn try_fetch_with_strategy(&self, url: &str, strategy: u32) -> AppResult<u32> {
        let mut request = self.client
            .get(url)
            .timeout(Duration::from_secs(self.timeout_seconds));

        // Different strategies for different attempts
        match strategy {
            1 => {
                // Strategy 1: Standard request
                request = request.header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36");
            }
            2 => {
                // Strategy 2: With additional headers to mimic browser
                request = request
                    .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                    .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
                    .header("Accept-Language", "en-US,en;q=0.5")
                    .header("Accept-Encoding", "gzip, deflate, br")
                    .header("DNT", "1")
                    .header("Connection", "keep-alive")
                    .header("Upgrade-Insecure-Requests", "1");
            }
            3 => {
                // Strategy 3: Try to access a potential API endpoint or different URL
                // Since the portal loads data dynamically, let's try some common patterns
                let base_url = "https://portal.withorb.com";
                let token = url.split("token=").nth(1).unwrap_or("");

                // Try different potential endpoints
                let api_urls = [
                    format!("{}/api/customer/balance?token={}", base_url, token),
                    format!("{}/api/v1/balance?token={}", base_url, token),
                    format!("{}/api/balance?token={}", base_url, token),
                    format!("{}/customer/data?token={}", base_url, token),
                ];

                for api_url in &api_urls {
                    tracing::debug!("Trying API endpoint: {}", api_url);
                    if let Ok(response) = self.client
                        .get(api_url)
                        .timeout(Duration::from_secs(self.timeout_seconds))
                        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
                        .send()
                        .await
                    {
                        if response.status().is_success() {
                            if let Ok(content) = response.text().await {
                                tracing::info!("Found API endpoint: {} with content length: {}", api_url, content.len());
                                if let Ok(balance) = self.parse_balance_from_html(&content) {
                                    return Ok(balance);
                                }
                            }
                        }
                    }
                }

                // Fallback to original URL with cache busting
                let cache_buster = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let url_with_cache_buster = format!("{}&_cb={}", url, cache_buster);
                request = self.client
                    .get(&url_with_cache_buster)
                    .timeout(Duration::from_secs(self.timeout_seconds))
                    .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36");
            }
            _ => {}
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(AppError::Scraping(format!(
                "HTTP error: {} - {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown error")
            )));
        }

        let html_content = response.text().await?;
        tracing::info!("Strategy {} - Received HTML content length: {}", strategy, html_content.len());

        self.parse_balance_from_html(&html_content)
    }

    async fn try_fetch_balance_from_api(&self, token: &str) -> AppResult<u32> {
        tracing::info!("Fetching balance from orb API using token");

        // First, get the list of customers to find the current customer
        let customers_response = self.client
            .get("https://api.withorb.com/v1/customers")
            .header("Authorization", format!("Bearer {}", token))
            .timeout(Duration::from_secs(self.timeout_seconds))
            .send()
            .await?;

        if !customers_response.status().is_success() {
            return Err(AppError::Scraping(format!(
                "API error: {} - {}",
                customers_response.status(),
                customers_response.status().canonical_reason().unwrap_or("Unknown error")
            )));
        }

        let customers_json: serde_json::Value = customers_response.json().await?;
        tracing::debug!("Customers API response: {}", customers_json);

        // Extract balance from the first customer (assuming single customer account)
        if let Some(customers) = customers_json["data"].as_array() {
            if let Some(customer) = customers.first() {
                if let Some(balance_str) = customer["balance"].as_str() {
                    // Parse balance string (could be "123.45" format)
                    let balance_float: f64 = balance_str.parse()
                        .map_err(|_| AppError::Scraping(format!("Invalid balance format: {}", balance_str)))?;

                    let balance = balance_float.round() as u32;
                    tracing::info!("✅ Found balance from API: {} (from string: '{}')", balance, balance_str);
                    return Ok(balance);
                }
            }
        }

        Err(AppError::Scraping("Could not find balance in API response".to_string()))
    }

    async fn try_fetch_balance(&self, url: &str) -> AppResult<u32> {
        tracing::info!("Fetching balance from: {}", url);

        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(self.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Scraping(format!(
                "HTTP error: {} - {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown error")
            )));
        }

        let html_content = response.text().await?;
        tracing::info!("Received HTML content length: {}", html_content.len());

        // Log first 1000 characters to see what we're getting
        let preview = if html_content.len() > 1000 {
            &html_content[..1000]
        } else {
            &html_content
        };
        tracing::info!("HTML preview: {}", preview);

        self.parse_balance_from_html(&html_content)
    }
    
    fn parse_balance_from_html(&self, html: &str) -> AppResult<u32> {
        let document = Html::parse_document(html);

        // For Next.js apps, first check if this is just the loading shell
        if html.contains("__NEXT_DATA__") && html.len() < 5000 {
            tracing::warn!("Received Next.js loading shell, content may not be fully loaded");
            // Still try to parse in case there's embedded data
        }

        // Strategy 1: Look for Next.js data in script tags first
        if let Some(balance) = self.extract_balance_from_nextjs_data(html) {
            return Ok(balance);
        }

        // Strategy 2: Look for "Credit balance" text and extract nearby numbers
        if let Some(balance) = self.extract_balance_strategy_1(&document) {
            return Ok(balance);
        }

        // Strategy 3: Look for specific patterns in the HTML text
        if let Some(balance) = self.extract_balance_strategy_2(&document) {
            return Ok(balance);
        }

        // Strategy 4: Look for common balance display patterns in elements
        if let Some(balance) = self.extract_balance_strategy_3(&document) {
            return Ok(balance);
        }

        // Strategy 5: Search in script tags for JSON data
        if let Some(balance) = self.extract_balance_from_scripts(&document) {
            return Ok(balance);
        }

        // Strategy 6: Search in raw HTML for patterns
        if let Some(balance) = self.extract_balance_from_raw_html(html) {
            return Ok(balance);
        }

        Err(AppError::Scraping("Could not find credit balance in page".to_string()))
    }
    
    fn extract_balance_strategy_1(&self, document: &Html) -> Option<u32> {
        // Look for elements containing "Credit balance" text
        let credit_balance_selector = Selector::parse("*").ok()?;
        
        for element in document.select(&credit_balance_selector) {
            let text = element.text().collect::<String>();
            if text.to_lowercase().contains("credit balance") {
                // Look for numbers in this element and its siblings/children
                if let Some(balance) = self.extract_number_from_text(&text) {
                    return Some(balance);
                }
                
                // Check parent element
                if let Some(parent) = element.parent() {
                    if let Some(parent_element) = parent.value().as_element() {
                        let parent_text = element.text().collect::<String>();
                        if let Some(balance) = self.extract_number_from_text(&parent_text) {
                            return Some(balance);
                        }
                    }
                }

                // Check next siblings
                for sibling in element.next_siblings() {
                    if let Some(sibling_element) = sibling.value().as_element() {
                        let sibling_text = element.text().collect::<String>();
                        if let Some(balance) = self.extract_number_from_text(&sibling_text) {
                            return Some(balance);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn extract_balance_strategy_2(&self, document: &Html) -> Option<u32> {
        // Look for orb-specific patterns and common balance patterns
        let patterns = [
            // orb-specific patterns
            r"(\d{1,3}(?:,\d{3})*)\s*(?:User\s*Messages?|Credits?|Messages?)",
            r"Credit\s*balance[:\s]*(\d{1,3}(?:,\d{3})*)",
            r"Balance[:\s]*(\d{1,3}(?:,\d{3})*)",
            r"(\d{1,3}(?:,\d{3})*)\s*remaining",
            r"Available[:\s]*(\d{1,3}(?:,\d{3})*)",
            r"(\d{1,3}(?:,\d{3})*)\s*(?:credits?|units?|tokens?)",
            // JSON patterns (in case data is in script tags)
            r#""balance"[:\s]*(\d+)"#,
            r#""credits?"[:\s]*(\d+)"#,
            r#""remaining"[:\s]*(\d+)"#,
            // Generic number patterns (last resort)
            r"(\d{1,3}(?:,\d{3})+)", // Numbers with commas like 2,683
        ];

        let full_text = document.root_element().text().collect::<String>();
        tracing::info!("Full page text length: {} chars", full_text.len());

        // Safely get first 1000 chars, handling UTF-8 boundaries
        let preview_len = full_text.len().min(1000);
        let mut safe_end = preview_len;
        while safe_end > 0 && !full_text.is_char_boundary(safe_end) {
            safe_end -= 1;
        }
        tracing::info!("Full page text (first {} chars): {}", safe_end, &full_text[..safe_end]);

        for (i, pattern) in patterns.iter().enumerate() {
            tracing::debug!("Trying pattern {}: {}", i + 1, pattern);
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(&full_text) {
                    if let Some(number_str) = captures.get(1) {
                        if let Some(number) = self.parse_number_string(number_str.as_str()) {
                            tracing::info!("✅ Found balance using pattern '{}': {} (from text: '{}')", pattern, number, number_str.as_str());
                            return Some(number);
                        }
                    }
                }
            }
        }

        tracing::warn!("❌ No balance found with any pattern");
        None
    }

    fn extract_balance_from_nextjs_data(&self, html: &str) -> Option<u32> {
        tracing::debug!("Searching for Next.js data");

        // Look for __NEXT_DATA__ script tag
        if let Some(start) = html.find("__NEXT_DATA__") {
            if let Some(json_start) = html[start..].find('{') {
                let json_start = start + json_start;
                if let Some(json_end) = html[json_start..].find("</script>") {
                    let json_end = json_start + json_end;
                    let json_str = &html[json_start..json_end];

                    tracing::debug!("Found Next.js data: {}", &json_str[..json_str.len().min(200)]);

                    // Try to parse as JSON and look for balance-related fields
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                        return self.search_json_for_balance(&json_value);
                    }
                }
            }
        }

        None
    }

    fn search_json_for_balance(&self, json: &serde_json::Value) -> Option<u32> {
        // Recursively search through JSON for balance-related fields
        match json {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    let key_lower = key.to_lowercase();
                    if key_lower.contains("balance") || key_lower.contains("credit") || key_lower.contains("amount") {
                        if let Some(balance) = self.extract_number_from_json_value(value) {
                            tracing::info!("✅ Found balance in JSON key '{}': {}", key, balance);
                            return Some(balance);
                        }
                    }

                    // Recursively search in nested objects/arrays
                    if let Some(balance) = self.search_json_for_balance(value) {
                        return Some(balance);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(balance) = self.search_json_for_balance(item) {
                        return Some(balance);
                    }
                }
            }
            _ => {}
        }

        None
    }

    fn extract_number_from_json_value(&self, value: &serde_json::Value) -> Option<u32> {
        match value {
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Some(f.round() as u32)
                } else if let Some(i) = n.as_u64() {
                    Some(i as u32)
                } else {
                    None
                }
            }
            serde_json::Value::String(s) => {
                // Try to parse string as number
                if let Ok(f) = s.parse::<f64>() {
                    Some(f.round() as u32)
                } else {
                    self.parse_number_string(s)
                }
            }
            _ => None,
        }
    }

    fn extract_balance_strategy_3(&self, document: &Html) -> Option<u32> {
        // Look for elements with specific classes or IDs that might contain balance
        let selectors = [
            "[class*='balance']",
            "[class*='credit']",
            "[id*='balance']",
            "[id*='credit']",
            "[data-testid*='balance']",
            "[data-testid*='credit']",
        ];
        
        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<String>();
                    if let Some(balance) = self.extract_number_from_text(&text) {
                        return Some(balance);
                    }
                }
            }
        }
        
        None
    }
    
    fn extract_number_from_text(&self, text: &str) -> Option<u32> {
        // First try to extract from the specific orb pattern: "Credit balance: 2,666 User Messages"
        if let Some(balance) = self.extract_orb_balance_pattern(text) {
            return Some(balance);
        }

        // Look for patterns like "2,683" or "2683"
        let number_patterns = [
            r"(\d{1,3}(?:,\d{3})+)", // Numbers with commas like 2,683
            r"(\d{4,})",             // Large numbers without commas
        ];

        for pattern in &number_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                for captures in regex.captures_iter(text) {
                    if let Some(number_str) = captures.get(1) {
                        if let Some(number) = self.parse_number_string(number_str.as_str()) {
                            // Sanity check: balance should be reasonable (between 0 and 1 million)
                            if number <= 1_000_000 {
                                return Some(number);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_orb_balance_pattern(&self, text: &str) -> Option<u32> {
        // Look for the specific orb pattern: "Credit balance: 2,666 User Messages"
        let orb_patterns = [
            r"Credit balance:\s*(\d{1,3}(?:,\d{3})*)\s*User Messages",
            r"Credit balance:\s*(\d{1,3}(?:,\d{3})*)",
            r"(\d{1,3}(?:,\d{3})*)\s*User Messages",
            r"balance:\s*(\d{1,3}(?:,\d{3})*)",
        ];

        for pattern in &orb_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(text) {
                    if let Some(number_str) = captures.get(1) {
                        if let Some(number) = self.parse_number_string(number_str.as_str()) {
                            tracing::info!("✅ Extracted balance using orb pattern '{}': {} from text: '{}'",
                                         pattern, number, text);
                            return Some(number);
                        }
                    }
                }
            }
        }

        None
    }
    
    fn parse_number_string(&self, s: &str) -> Option<u32> {
        // Remove commas and parse
        let cleaned = s.replace(',', "");
        cleaned.parse::<u32>().ok()
    }

    fn extract_balance_from_scripts(&self, document: &Html) -> Option<u32> {
        let script_selector = Selector::parse("script").ok()?;

        for script in document.select(&script_selector) {
            let script_content = script.inner_html();
            tracing::debug!("Checking script content (length: {})", script_content.len());

            // Look for JSON patterns in script tags
            let json_patterns = [
                r#""balance"[:\s]*(\d+)"#,
                r#""credits?"[:\s]*(\d+)"#,
                r#""remaining"[:\s]*(\d+)"#,
                r#""amount"[:\s]*(\d+)"#,
                r#""value"[:\s]*(\d+)"#,
            ];

            for pattern in &json_patterns {
                if let Ok(regex) = regex::Regex::new(pattern) {
                    if let Some(captures) = regex.captures(&script_content) {
                        if let Some(number_str) = captures.get(1) {
                            if let Some(number) = self.parse_number_string(number_str.as_str()) {
                                tracing::info!("✅ Found balance in script tag using pattern '{}': {}", pattern, number);
                                return Some(number);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_balance_from_raw_html(&self, html: &str) -> Option<u32> {
        tracing::debug!("Searching raw HTML for balance patterns");

        // Search for patterns in the raw HTML (including attributes, etc.)
        let raw_patterns = [
            r#"data-balance[="][^"]*(\d{1,3}(?:,\d{3})*)"#,
            r#"data-credits?[="][^"]*(\d{1,3}(?:,\d{3})*)"#,
            r#"value[="][^"]*(\d{1,3}(?:,\d{3})*)"#,
            r#"aria-label[="][^"]*(\d{1,3}(?:,\d{3})*)"#,
            r#"title[="][^"]*(\d{1,3}(?:,\d{3})*)"#,
        ];

        for pattern in &raw_patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(html) {
                    if let Some(number_str) = captures.get(1) {
                        if let Some(number) = self.parse_number_string(number_str.as_str()) {
                            tracing::info!("✅ Found balance in raw HTML using pattern '{}': {}", pattern, number);
                            return Some(number);
                        }
                    }
                }
            }
        }

        None
    }
    
    pub async fn validate_token(&self, token: &str) -> AppResult<bool> {
        let url = format!("https://portal.withorb.com/view?token={}", token);
        
        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        // Check if we get a successful response and the page contains expected content
        if response.status().is_success() {
            let html = response.text().await?;
            // Look for indicators that this is a valid orb portal page
            let indicators = ["credit", "balance", "orb", "portal"];
            let html_lower = html.to_lowercase();
            
            let has_indicators = indicators.iter().any(|&indicator| html_lower.contains(indicator));
            Ok(has_indicators)
        } else {
            Ok(false)
        }
    }
}

// Add regex dependency to Cargo.toml
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_balance_from_html() {
        let scraper = orbScraper::new().await.unwrap();

        let test_html = r#"
            <div>
                <h2>Credit balance</h2>
                <span>2,683 User Messages</span>
            </div>
        "#;

        let balance = scraper.parse_balance_from_html(test_html).unwrap();
        assert_eq!(balance, 2683);
    }

    #[tokio::test]
    async fn test_extract_number_from_text() {
        let scraper = orbScraper::new().await.unwrap();

        assert_eq!(scraper.extract_number_from_text("2,683 User Messages"), Some(2683));
        assert_eq!(scraper.extract_number_from_text("Balance: 1,234"), Some(1234));
        assert_eq!(scraper.extract_number_from_text("5000 credits remaining"), Some(5000));
        assert_eq!(scraper.extract_number_from_text("No numbers here"), None);
    }
}
