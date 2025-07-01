<script>
  import '../app.css';
  import { page } from '$app/stores';
  
  // Handle keyboard navigation globally
  function handleKeydown(e) {
    // "/" focuses search unless already in an input
    if (e.key === '/' && !['INPUT', 'TEXTAREA'].includes(e.target.tagName)) {
      e.preventDefault();
      const searchInput = document.querySelector('#global-search');
      if (searchInput) searchInput.focus();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app">
  <nav>
    <div class="container">
      <ul>
        <li><a href="/" class:active={$page.url.pathname === '/'}>Search</a></li>
        <li><a href="/conversations" class:active={$page.url.pathname.startsWith('/conversations')}>Conversations</a></li>
      </ul>
    </div>
  </nav>
  
  <main>
    <slot />
  </main>
</div>

<style>
  .app {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
  
  main {
    flex: 1;
  }
</style>