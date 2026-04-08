const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';

export interface SystemStatus {
  auth_api: boolean;
  rpc_proxy: boolean;
  payment_system: boolean;
  rpc_url?: string;
}

export interface NetworkInfo {
  network: string;
}

export interface LoginRequest {
  wallet_address: string;
  message: string;
  signature: string;
}

export interface LoginResponse {
  success: boolean;
  user_id: string;
  wallet_address: string;
}

export interface CurrentUser {
  user_id: string;
  wallet_address: string;
  credits_balance: number;
  tier: string;
}

export interface SignupRequest {
  wallet_address: string;
  message: string;
  signature: string;
}

export interface SignupResponse {
  api_key: string;
  user_id: string;
  credits_balance: number;
}

export interface PricingPackage {
  package: string;
  token_amount: number;
  token_amount_formatted: string;
  credits: number;
  credits_formatted: string;
}

export interface TokenInfo {
  name: string;
  symbol: string;
  mint: string;
  logo?: string;
  decimals: number;
}

export interface PricingResponse {
  enabled: boolean;
  packages: PricingPackage[];
  token_info?: TokenInfo;
}

export interface PaymentRequest {
  api_key: string;
  package: string;
  token_type: string;
}

export interface PaymentResponse {
  payment_id: string;
  payment_url: string;
  amount: number;
  credits: number;
}

export interface PaymentVerification {
  payment_id: string;
  signature: string;
}

export interface PaymentVerificationResponse {
  verified: boolean;
  credits_purchased?: number;
  new_balance?: number;
}

export interface UserStats {
  api_key: string;
  wallet_address: string;
  credits_balance: number;
  credits_used_this_month: number;
  total_requests: number;
  total_blocked: number;
  total_warnings: number;
  blocking_threshold: number;
  notifications_enabled: boolean;
}

export interface GlobalStats {
  total_requests: number;
  total_blocked: number;
  total_warnings: number;
  requests_per_second: number;
}

export interface MatchedRuleInfo {
  rule_id: string;
  rule_name: string;
  action: string;
  weight: number;
  message: string;
}

export interface SecurityEvent {
  id: string;
  event_type: string;
  severity: string;
  threat_category?: string;
  description?: string;
  created_at: string;
  signature?: string;
  wallet?: string;
  method?: string;
  summary?: string;
  programs?: string[];
  amount?: string;
  risk_score?: number;
  rule_matches?: number;
  matched_rule_ids?: string[]; // Just IDs for performance
}

export interface ActiveRule {
  id: string;
  name: string;
  description: string;
  action: string;
  severity: string;
  enabled: boolean;
  hit_count: number;
}

export interface ActiveRulesResponse {
  rules_source: string;
  total_rules: number;
  active_rules: number;
  total_hits: number;
  rules: ActiveRule[];
}

class ApiService {
  private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      ...options,
      credentials: 'include', // Include cookies in requests
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Request failed' }));
      throw new Error(error.error || `HTTP ${response.status}`);
    }

    return response.json();
  }

  // Session-based auth (for dashboard)
  async login(data: LoginRequest): Promise<LoginResponse> {
    return this.request('/auth/login', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async getCurrentUser(): Promise<CurrentUser> {
    return this.request('/auth/me');
  }

  async logout(): Promise<void> {
    await this.request('/auth/logout', {
      method: 'POST',
    });
  }

  async getMyStats(): Promise<UserStats> {
    return this.request('/dashboard/stats');
  }

  async getMyEvents(limit?: number, offset?: number): Promise<SecurityEvent[]> {
    const params = new URLSearchParams();
    if (limit !== undefined) params.append('limit', limit.toString());
    if (offset !== undefined) params.append('offset', offset.toString());
    const query = params.toString();
    return this.request(`/dashboard/events${query ? `?${query}` : ''}`);
  }

  async getActiveRules(): Promise<ActiveRulesResponse> {
    return this.request('/dashboard/rules');
  }

  async regenerateMyApiKey(): Promise<{ api_key: string; warning: string }> {
    return this.request('/auth/api-key/regenerate', {
      method: 'POST',
    });
  }

  async updateBlockingThreshold(threshold: number): Promise<{ blocking_threshold: number }> {
    return this.request('/dashboard/threshold', {
      method: 'PUT',
      body: JSON.stringify({ threshold }),
    });
  }

  async toggleNotifications(enabled: boolean): Promise<{ notifications_enabled: boolean }> {
    return this.request('/dashboard/notifications', {
      method: 'PUT',
      body: JSON.stringify({ enabled }),
    });
  }

  async getStatus(): Promise<SystemStatus> {
    return this.request('/health');
  }

  async getNetworkInfo(): Promise<NetworkInfo> {
    return this.request('/system/network');
  }

  async signup(data: SignupRequest): Promise<SignupResponse> {
    return this.request('/signup', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async getPricing(): Promise<PricingResponse> {
    return this.request('/payment/pricing');
  }

  async createPayment(data: PaymentRequest): Promise<PaymentResponse> {
    return this.request('/payment/create', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async verifyPayment(data: PaymentVerification): Promise<PaymentVerificationResponse> {
    return this.request('/payment/verify', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async getUserStats(apiKey: string): Promise<UserStats> {
    return this.request(`/stats/user/${apiKey}`);
  }

  async getGlobalStats(): Promise<GlobalStats> {
    return this.request('/stats/global');
  }

  async getSecurityEvents(apiKey: string, limit?: number, offset?: number): Promise<SecurityEvent[]> {
    const params = new URLSearchParams();
    if (limit !== undefined) params.append('limit', limit.toString());
    if (offset !== undefined) params.append('offset', offset.toString());
    const query = params.toString();
    return this.request(`/stats/events/${apiKey}${query ? `?${query}` : ''}`);
  }

  async regenerateApiKey(apiKey: string): Promise<SignupResponse> {
    return this.request('/api-key/regenerate', {
      method: 'POST',
      body: JSON.stringify({ api_key: apiKey }),
    });
  }

  // Learning system endpoints
  async getCourses(): Promise<{ courses: any[]; total: number }> {
    return this.request('/learn/courses');
  }

  async getCourseBySlug(slug: string): Promise<any> {
    return this.request(`/learn/courses/slug/${slug}`);
  }

  async getMyProgress(): Promise<any[]> {
    return this.request('/learn/progress/me');
  }

  async getCourseProgress(courseId: string): Promise<any> {
    return this.request(`/learn/progress/course/${courseId}`);
  }

  async updateCourseProgress(courseId: string, data: { progress_data: any; completed?: boolean }): Promise<any> {
    return this.request(`/learn/progress/course/${courseId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async getMyBadges(): Promise<any[]> {
    return this.request('/learn/badges/me');
  }

  async getAllBadges(): Promise<any[]> {
    return this.request('/learn/badges');
  }
}

export const apiService = new ApiService();
