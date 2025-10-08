
        // Theme Management
        (function() {
            const html = document.documentElement;
            const themeToggle = document.getElementById('themeToggle');
            const themeIcon = document.getElementById('themeIcon');
            const themeText = document.getElementById('themeText');
            
            // Load saved theme or default to light
            const savedTheme = localStorage.getItem('theme') || 'light';
            setTheme(savedTheme);
            
            // Theme toggle click handler
            themeToggle.addEventListener('click', () => {
                const currentTheme = html.getAttribute('data-theme');
                const newTheme = currentTheme === 'light' ? 'dark' : 'light';
                setTheme(newTheme);
                localStorage.setItem('theme', newTheme);
            });
            
            function setTheme(theme) {
                html.setAttribute('data-theme', theme);
                if (theme === 'dark') {
                    themeIcon.textContent = '☀️';
                    themeText.textContent = 'Light';
                } else {
                    themeIcon.textContent = '🌙';
                    themeText.textContent = 'Dark';
                }
            }
        })();
    