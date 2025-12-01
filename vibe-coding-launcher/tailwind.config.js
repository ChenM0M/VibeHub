/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{js,ts,jsx,tsx}",
    ],
    darkMode: 'class',
    theme: {
        extend: {
            colors: {
                notion: {
                    bg: {
                        light: '#FFFFFF',
                        dark: '#191919',
                    },
                    text: {
                        primary: {
                            light: '#37352F',
                            dark: '#E9E9E7',
                        },
                        secondary: {
                            light: '#787774',
                            dark: '#9B9A97',
                        },
                    },
                    border: {
                        light: '#E9E9E7',
                        dark: '#373737',
                    },
                    hover: {
                        light: '#F7F6F3',
                        dark: '#2F2F2F',
                    },
                    blue: '#2EAADC',
                    red: '#D44C47',
                    green: '#448361',
                    orange: '#D9730D',
                    purple: '#9065B0',
                    pink: '#C14C8A',
                },
            },
            fontFamily: {
                sans: ['Inter', 'system-ui', 'sans-serif'],
            },
            animation: {
                'fade-in': 'fadeIn 0.2s ease-in-out',
                'slide-up': 'slideUp 0.3s ease-out',
                'slide-down': 'slideDown 0.3s ease-out',
            },
            keyframes: {
                fadeIn: {
                    '0%': { opacity: '0' },
                    '100%': { opacity: '1' },
                },
                slideUp: {
                    '0%': { transform: 'translateY(10px)', opacity: '0' },
                    '100%': { transform: 'translateY(0)', opacity: '1' },
                },
                slideDown: {
                    '0%': { transform: 'translateY(-10px)', opacity: '0' },
                    '100%': { transform: 'translateY(0)', opacity: '1' },
                },
            },
        },
    },
    plugins: [],
}
