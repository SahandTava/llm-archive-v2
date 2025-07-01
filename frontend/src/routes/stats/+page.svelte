<script lang="ts">
    import { onMount } from 'svelte';
    
    interface Stats {
        total_conversations: number;
        total_messages: number;
        providers: Record<string, number>;
        models: Record<string, number>;
        messages_by_role: Record<string, number>;
        activity_by_hour: number[];
        growth_by_month: Array<{month: string, count: number}>;
    }
    
    let stats: Stats | null = null;
    let loading = true;
    
    onMount(async () => {
        try {
            const response = await fetch('/api/stats');
            stats = await response.json();
        } catch (error) {
            console.error('Failed to load stats:', error);
        } finally {
            loading = false;
        }
    });
    
    // Create simple bar chart
    function barWidth(value: number, max: number): string {
        return `${(value / max) * 100}%`;
    }
</script>

<div class="stats-container">
    <h1>Conversation Statistics</h1>
    
    {#if loading}
        <p>Loading statistics...</p>
    {:else if stats}
        <div class="stats-grid">
            <!-- Overview Cards -->
            <div class="stat-card">
                <h2>Total Conversations</h2>
                <div class="stat-value">{stats.total_conversations.toLocaleString()}</div>
            </div>
            
            <div class="stat-card">
                <h2>Total Messages</h2>
                <div class="stat-value">{stats.total_messages.toLocaleString()}</div>
            </div>
            
            <div class="stat-card">
                <h2>Avg Messages/Conversation</h2>
                <div class="stat-value">
                    {(stats.total_messages / stats.total_conversations).toFixed(1)}
                </div>
            </div>
        </div>
        
        <!-- Provider Distribution -->
        <section class="chart-section">
            <h2>Conversations by Provider</h2>
            <div class="bar-chart">
                {#each Object.entries(stats.providers).sort((a, b) => b[1] - a[1]) as [provider, count]}
                    <div class="bar-row">
                        <div class="bar-label">{provider}</div>
                        <div class="bar-container">
                            <div 
                                class="bar" 
                                style="width: {barWidth(count, Math.max(...Object.values(stats.providers)))}"
                            >
                                <span class="bar-value">{count.toLocaleString()}</span>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        </section>
        
        <!-- Model Distribution -->
        <section class="chart-section">
            <h2>Top 10 Models</h2>
            <div class="bar-chart">
                {#each Object.entries(stats.models)
                    .sort((a, b) => b[1] - a[1])
                    .slice(0, 10) as [model, count]}
                    <div class="bar-row">
                        <div class="bar-label">{model}</div>
                        <div class="bar-container">
                            <div 
                                class="bar" 
                                style="width: {barWidth(count, Object.values(stats.models)[0])}"
                            >
                                <span class="bar-value">{count.toLocaleString()}</span>
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        </section>
        
        <!-- Message Roles -->
        <section class="chart-section">
            <h2>Messages by Role</h2>
            <div class="role-stats">
                {#each Object.entries(stats.messages_by_role) as [role, count]}
                    <div class="role-card">
                        <div class="role-name">{role}</div>
                        <div class="role-count">{count.toLocaleString()}</div>
                        <div class="role-percent">
                            {((count / stats.total_messages) * 100).toFixed(1)}%
                        </div>
                    </div>
                {/each}
            </div>
        </section>
        
        <!-- Activity Heatmap -->
        <section class="chart-section">
            <h2>Activity by Hour</h2>
            <div class="heatmap">
                {#each stats.activity_by_hour as count, hour}
                    <div 
                        class="heatmap-cell"
                        style="opacity: {count / Math.max(...stats.activity_by_hour)}"
                        title="{hour}:00 - {count} messages"
                    >
                        <span class="hour-label">{hour}</span>
                    </div>
                {/each}
            </div>
        </section>
    {:else}
        <p>Failed to load statistics.</p>
    {/if}
</div>

<style>
    .stats-container {
        max-width: 1200px;
        margin: 0 auto;
        padding: 2rem;
    }
    
    h1 {
        text-align: center;
        margin-bottom: 2rem;
    }
    
    .stats-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
        gap: 1.5rem;
        margin-bottom: 3rem;
    }
    
    .stat-card {
        background: #f5f5f5;
        padding: 1.5rem;
        border-radius: 8px;
        text-align: center;
    }
    
    .stat-card h2 {
        margin: 0 0 0.5rem 0;
        font-size: 1rem;
        color: #666;
    }
    
    .stat-value {
        font-size: 2.5rem;
        font-weight: bold;
        color: #333;
    }
    
    .chart-section {
        margin-bottom: 3rem;
    }
    
    .chart-section h2 {
        margin-bottom: 1rem;
    }
    
    .bar-chart {
        background: #f9f9f9;
        padding: 1rem;
        border-radius: 8px;
    }
    
    .bar-row {
        display: flex;
        align-items: center;
        margin-bottom: 0.5rem;
    }
    
    .bar-label {
        width: 150px;
        font-size: 0.9rem;
        text-align: right;
        padding-right: 1rem;
    }
    
    .bar-container {
        flex: 1;
        background: #e0e0e0;
        border-radius: 4px;
        height: 24px;
        position: relative;
    }
    
    .bar {
        background: #007bff;
        height: 100%;
        border-radius: 4px;
        transition: width 0.3s ease;
        display: flex;
        align-items: center;
        padding-right: 0.5rem;
        justify-content: flex-end;
    }
    
    .bar-value {
        color: white;
        font-size: 0.8rem;
        font-weight: bold;
    }
    
    .role-stats {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
        gap: 1rem;
    }
    
    .role-card {
        background: #f5f5f5;
        padding: 1rem;
        border-radius: 8px;
        text-align: center;
    }
    
    .role-name {
        font-weight: bold;
        margin-bottom: 0.5rem;
    }
    
    .role-count {
        font-size: 1.5rem;
        color: #007bff;
    }
    
    .role-percent {
        font-size: 0.9rem;
        color: #666;
    }
    
    .heatmap {
        display: grid;
        grid-template-columns: repeat(24, 1fr);
        gap: 2px;
        background: #f0f0f0;
        padding: 1rem;
        border-radius: 8px;
    }
    
    .heatmap-cell {
        aspect-ratio: 1;
        background: #007bff;
        border-radius: 2px;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        transition: transform 0.1s;
    }
    
    .heatmap-cell:hover {
        transform: scale(1.1);
    }
    
    .hour-label {
        font-size: 0.7rem;
        color: white;
        font-weight: bold;
    }
</style>