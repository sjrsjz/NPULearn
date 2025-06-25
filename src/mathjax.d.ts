declare global {
    interface Window {
        hljs: any;
        MathJax: any;
    }
}

declare module 'mathjax/es5/tex-svg';

export { };