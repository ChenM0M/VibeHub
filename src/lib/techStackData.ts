// Tech Stack and Icon Presets Data

export interface TechStackItem {
    id: string;
    name: string;
    icon: string;  // CDN URL for the icon
    color: string; // Badge color
    category: 'language' | 'framework' | 'database' | 'tool' | 'cloud';
}

// Using DevIcons CDN: https://cdn.jsdelivr.net/gh/devicons/devicon/icons/{name}/{name}-original.svg

export const TECH_STACK_PRESETS: TechStackItem[] = [
    // Languages
    { id: 'javascript', name: 'JavaScript', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/javascript/javascript-original.svg', color: '#F7DF1E', category: 'language' },
    { id: 'typescript', name: 'TypeScript', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/typescript/typescript-original.svg', color: '#3178C6', category: 'language' },
    { id: 'python', name: 'Python', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/python/python-original.svg', color: '#3776AB', category: 'language' },
    { id: 'rust', name: 'Rust', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/rust/rust-original.svg', color: '#000000', category: 'language' },
    { id: 'go', name: 'Go', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/go/go-original-wordmark.svg', color: '#00ADD8', category: 'language' },
    { id: 'java', name: 'Java', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/java/java-original.svg', color: '#ED8B00', category: 'language' },
    { id: 'csharp', name: 'C#', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/csharp/csharp-original.svg', color: '#512BD4', category: 'language' },
    { id: 'cpp', name: 'C++', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/cplusplus/cplusplus-original.svg', color: '#00599C', category: 'language' },
    { id: 'php', name: 'PHP', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/php/php-original.svg', color: '#777BB4', category: 'language' },
    { id: 'ruby', name: 'Ruby', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/ruby/ruby-original.svg', color: '#CC342D', category: 'language' },
    { id: 'swift', name: 'Swift', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/swift/swift-original.svg', color: '#F05138', category: 'language' },
    { id: 'kotlin', name: 'Kotlin', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/kotlin/kotlin-original.svg', color: '#7F52FF', category: 'language' },
    { id: 'dart', name: 'Dart', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/dart/dart-original.svg', color: '#0175C2', category: 'language' },
    { id: 'html5', name: 'HTML5', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/html5/html5-original.svg', color: '#E34F26', category: 'language' },
    { id: 'css3', name: 'CSS3', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/css3/css3-original.svg', color: '#1572B6', category: 'language' },

    // Frontend Frameworks
    { id: 'react', name: 'React', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/react/react-original.svg', color: '#61DAFB', category: 'framework' },
    { id: 'vue', name: 'Vue.js', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/vuejs/vuejs-original.svg', color: '#4FC08D', category: 'framework' },
    { id: 'angular', name: 'Angular', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/angularjs/angularjs-original.svg', color: '#DD0031', category: 'framework' },
    { id: 'svelte', name: 'Svelte', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/svelte/svelte-original.svg', color: '#FF3E00', category: 'framework' },
    { id: 'nextjs', name: 'Next.js', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/nextjs/nextjs-original.svg', color: '#000000', category: 'framework' },
    { id: 'nuxtjs', name: 'Nuxt.js', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/nuxtjs/nuxtjs-original.svg', color: '#00DC82', category: 'framework' },
    { id: 'solidjs', name: 'SolidJS', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/solidjs/solidjs-original.svg', color: '#2C4F7C', category: 'framework' },

    // Backend Frameworks
    { id: 'nodejs', name: 'Node.js', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/nodejs/nodejs-original.svg', color: '#339933', category: 'framework' },
    { id: 'express', name: 'Express', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/express/express-original.svg', color: '#000000', category: 'framework' },
    { id: 'nestjs', name: 'NestJS', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/nestjs/nestjs-original.svg', color: '#E0234E', category: 'framework' },
    { id: 'django', name: 'Django', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/django/django-plain.svg', color: '#092E20', category: 'framework' },
    { id: 'flask', name: 'Flask', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/flask/flask-original.svg', color: '#000000', category: 'framework' },
    { id: 'fastapi', name: 'FastAPI', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/fastapi/fastapi-original.svg', color: '#009688', category: 'framework' },
    { id: 'spring', name: 'Spring', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/spring/spring-original.svg', color: '#6DB33F', category: 'framework' },
    { id: 'rails', name: 'Rails', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/rails/rails-original-wordmark.svg', color: '#CC0000', category: 'framework' },
    { id: 'laravel', name: 'Laravel', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/laravel/laravel-original.svg', color: '#FF2D20', category: 'framework' },

    // Mobile
    { id: 'flutter', name: 'Flutter', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/flutter/flutter-original.svg', color: '#02569B', category: 'framework' },
    { id: 'reactnative', name: 'React Native', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/react/react-original.svg', color: '#61DAFB', category: 'framework' },

    // Databases
    { id: 'mysql', name: 'MySQL', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/mysql/mysql-original.svg', color: '#4479A1', category: 'database' },
    { id: 'postgresql', name: 'PostgreSQL', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/postgresql/postgresql-original.svg', color: '#4169E1', category: 'database' },
    { id: 'mongodb', name: 'MongoDB', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/mongodb/mongodb-original.svg', color: '#47A248', category: 'database' },
    { id: 'redis', name: 'Redis', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/redis/redis-original.svg', color: '#DC382D', category: 'database' },
    { id: 'sqlite', name: 'SQLite', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/sqlite/sqlite-original.svg', color: '#003B57', category: 'database' },
    { id: 'firebase', name: 'Firebase', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/firebase/firebase-plain.svg', color: '#FFCA28', category: 'database' },

    // DevOps & Tools
    { id: 'docker', name: 'Docker', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/docker/docker-original.svg', color: '#2496ED', category: 'tool' },
    { id: 'kubernetes', name: 'Kubernetes', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/kubernetes/kubernetes-plain.svg', color: '#326CE5', category: 'tool' },
    { id: 'git', name: 'Git', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/git/git-original.svg', color: '#F05032', category: 'tool' },
    { id: 'github', name: 'GitHub', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/github/github-original.svg', color: '#181717', category: 'tool' },
    { id: 'gitlab', name: 'GitLab', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/gitlab/gitlab-original.svg', color: '#FC6D26', category: 'tool' },
    { id: 'nginx', name: 'Nginx', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/nginx/nginx-original.svg', color: '#009639', category: 'tool' },
    { id: 'graphql', name: 'GraphQL', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/graphql/graphql-plain.svg', color: '#E10098', category: 'tool' },
    { id: 'webpack', name: 'Webpack', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/webpack/webpack-original.svg', color: '#8DD6F9', category: 'tool' },
    { id: 'vite', name: 'Vite', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/vitejs/vitejs-original.svg', color: '#646CFF', category: 'tool' },
    { id: 'tauri', name: 'Tauri', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tauri/tauri-original.svg', color: '#FFC131', category: 'tool' },
    { id: 'electron', name: 'Electron', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/electron/electron-original.svg', color: '#47848F', category: 'tool' },
    { id: 'tailwindcss', name: 'Tailwind CSS', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/tailwindcss/tailwindcss-original.svg', color: '#06B6D4', category: 'tool' },
    { id: 'sass', name: 'Sass', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/sass/sass-original.svg', color: '#CC6699', category: 'tool' },

    // Cloud
    { id: 'aws', name: 'AWS', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/amazonwebservices/amazonwebservices-plain-wordmark.svg', color: '#FF9900', category: 'cloud' },
    { id: 'azure', name: 'Azure', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/azure/azure-original.svg', color: '#0078D4', category: 'cloud' },
    { id: 'gcp', name: 'Google Cloud', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/googlecloud/googlecloud-original.svg', color: '#4285F4', category: 'cloud' },
    { id: 'vercel', name: 'Vercel', icon: 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons/vercel/vercel-original.svg', color: '#000000', category: 'cloud' },
];

// Project Icon Presets (same as tech stack icons, can be reused)
export const ICON_PRESETS = TECH_STACK_PRESETS.map(item => ({
    id: item.id,
    name: item.name,
    icon: item.icon,
    color: item.color,
}));

// Category labels for grouping
export const CATEGORY_LABELS: Record<string, string> = {
    language: '编程语言',
    framework: '框架',
    database: '数据库',
    tool: '工具',
    cloud: '云服务',
};

// Get tech stack item by ID
export function getTechStackById(id: string): TechStackItem | undefined {
    return TECH_STACK_PRESETS.find(item => item.id === id);
}

// Get tech stack items by IDs
export function getTechStackByIds(ids: string[]): TechStackItem[] {
    return ids.map(id => getTechStackById(id)).filter((item): item is TechStackItem => item !== undefined);
}
