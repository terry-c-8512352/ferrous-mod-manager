<script lang="ts">
    interface Props {
        message: string;
        variant?: 'error' | 'success';
        duration?: number;
        onclear: () => void;
    }

    let { message, variant = 'error', duration = 4000, onclear }: Props = $props();

    $effect(() => {
        if (!message) return;
        const t = setTimeout(onclear, duration);
        return () => clearTimeout(t);
    });
</script>

{#if message}
    <div class="toast" class:success={variant === 'success'} role="alert">
        <span class="toast-icon">{variant === 'success' ? '✓' : '⚠'}</span>
        <span class="toast-message">{message}</span>
        <button class="toast-close" onclick={onclear} aria-label="Dismiss">✕</button>
    </div>
{/if}

<style>
    .toast {
        position: fixed;
        bottom: 24px;
        right: 24px;
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 10px 14px;
        background: var(--code-bg);
        border: 1px solid var(--tertiary);
        border-radius: 4px;
        box-shadow: var(--shadow);
        color: var(--tertiary);
        font-size: 13px;
        z-index: 1000;
        max-width: 360px;
    }

    .toast.success {
        border-color: var(--accent);
        color: var(--accent);
    }

    .toast-icon {
        flex-shrink: 0;
    }

    .toast-message {
        flex: 1;
        color: var(--text-h);
    }

    .toast-close {
        flex-shrink: 0;
        background: none;
        border: none;
        color: var(--text);
        cursor: pointer;
        font-size: 12px;
        padding: 0 2px;
        line-height: 1;
    }

    .toast-close:hover {
        color: var(--text-h);
    }
</style>
