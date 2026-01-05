import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "ZyberLink",
  description: "Decentralized Multi-Prover Network for Privacy-Preserving Computations",
  cleanUrls: true,
  lastUpdated: true,
  ignoreDeadLinks: true,

  themeConfig: {
    socialLinks: [
      { icon: 'github', link: 'https://github.com/8ctag0n/13' }
    ],
    search: {
      provider: 'local'
    }
  },

  vite: {
    build: {
      chunkSizeWarningLimit: 1000
    }
  },

  locales: {
    root: {
      label: 'Home',
      lang: 'en'
    },
    en: {
      label: 'English',
      lang: 'en',
      link: '/en/',
      themeConfig: {
        nav: [
          { text: 'Getting Started', link: '/en/getting-started/quickstart' },
          { text: 'Guides', link: '/en/guides/' },
          { text: 'Architecture', link: '/en/architecture/overview' }
        ],
        sidebar: [
          {
            text: 'Getting Started',
            collapsed: false,
            items: [
              { text: 'Quickstart', link: '/en/getting-started/quickstart' },
              { text: 'Demo', link: '/en/getting-started/demo' },
              { text: 'Repositories', link: '/en/getting-started/repositories' }
            ]
          },
          {
            text: 'Verticals',
            collapsed: true,
            items: [
              { text: 'DeFi', link: '/en/verticals/defi' },
              { text: 'Governance', link: '/en/verticals/governance' },
              { text: 'Identity', link: '/en/verticals/identity' },
              { text: 'pBTCFi', link: '/en/verticals/pbtcfi' },
              { text: 'Private Lending', link: '/en/verticals/private_lending' }
            ]
          },
          {
            text: 'Architecture',
            collapsed: true,
            items: [
              { text: 'System Overview', link: '/en/architecture/overview' },
              { text: 'FHE Design', link: '/en/architecture/fhe-design' },
              { text: 'E2E FHE Flow', link: '/en/architecture/e2e-fhe-flow' },
              { text: 'Histogram Optimization', link: '/en/architecture/histogram-fhe-optimization' },
              { text: 'Tech Stack', link: '/en/architecture/tech-stack' },
              { text: 'Implementation Details', link: '/en/architecture/implementation-details' },
              { text: 'Attestation Service', link: '/en/architecture/attestation-service' },
              { text: 'Web App Integration', link: '/en/architecture/webapp-integration' },
              { text: 'wZEC Architecture', link: '/en/architecture/wzec-architecture' }
            ]
          },
          {
            text: 'Guides',
            collapsed: true,
            items: [
              { text: 'Private Analytics', link: '/en/guides/analytics-guide' },
              { text: 'Proof of Innocence', link: '/en/guides/proof-of-innocence-guide' },
              { text: 'SDK Integration', link: '/en/guides/sdk-integration' },
              { text: 'API Reference', link: '/en/guides/api-reference' },
              { text: 'Full API Reference', link: '/en/guides/api-reference-full' },
              { text: 'Jobs Endpoint', link: '/en/guides/api-jobs-endpoint' },
              { text: 'Price Slider', link: '/en/guides/price-slider-integration' },
              { text: 'Examples', link: '/en/guides/examples' },
              { text: 'Deployment', link: '/en/guides/deployment' },
              { text: 'Prover Setup', link: '/en/guides/prover-setup' },
              { text: 'Zyb CLI Guide', link: '/en/guides/cli-guide' },
              { text: 'ZK Circuits Guide', link: '/en/guides/zk-circuits' },
              { text: 'Web App Guide', link: '/en/guides/webapp-guide' },
              {
                text: 'Dynamic Pricing',
                collapsed: true,
                items: [
                    { text: 'Overview', link: '/en/guides/dynamic-pricing-overview' },
                    { text: 'Model', link: '/en/guides/dynamic-pricing-model' },
                    { text: 'Technical', link: '/en/guides/dynamic-pricing-technical' },
                    { text: 'Usage', link: '/en/guides/dynamic-pricing-usage' }
                ]
              },
              {
                text: 'wZEC Payment',
                collapsed: true,
                items: [
                    { text: 'User Guide', link: '/en/guides/wzec-user-guide' },
                    { text: 'Developer Guide', link: '/en/guides/wzec-developer-guide' },
                    { text: 'API Reference', link: '/en/guides/wzec-api-reference' },
                    { text: 'wZEC API', link: '/en/guides/wzec-api' },
                    { text: 'Testing Guide', link: '/en/guides/wzec-testing-guide' }
                ]
              }
            ]
          },
          {
            text: 'Reference Wallet (Experimental)',
            collapsed: true,
            items: [
              { text: 'Quickstart', link: '/en/wallet-extension/quickstart' },
              { text: 'Development', link: '/en/wallet-extension/development' },
              { text: 'Testing', link: '/en/wallet-extension/testing' },
              { text: 'Testnets', link: '/en/wallet-extension/testnets' }
            ]
          }
        ]
      }
    },
    es: {
      label: 'Español',
      lang: 'es',
      link: '/es/',
      themeConfig: {
        nav: [
          { text: 'Primeros Pasos', link: '/es/primeros-pasos/inicio-rapido' },
          { text: 'Guías', link: '/es/guias/' },
          { text: 'Arquitectura', link: '/es/arquitectura/vision-general' }
        ],
        sidebar: [
          {
            text: 'Primeros Pasos',
            collapsed: false,
            items: [
              { text: 'Inicio Rápido', link: '/es/primeros-pasos/inicio-rapido' },
              { text: 'Demo', link: '/es/primeros-pasos/demo' },
              { text: 'Repositorios', link: '/es/primeros-pasos/repositorios' }
            ]
          },
          {
            text: 'Arquitectura',
            collapsed: true,
            items: [
              { text: 'Visión General', link: '/es/arquitectura/vision-general' },
              { text: 'Diseño FHE', link: '/es/arquitectura/diseno-fhe' },
              { text: 'Flujo FHE E2E', link: '/es/arquitectura/flujo-fhe-e2e' },
              { text: 'Integración WebApp', link: '/es/arquitectura/integracion-webapp' },
              { text: 'Optimización Histograma', link: '/es/arquitectura/histograma-fhe-optimizaciones' },
              { text: 'Stack Tecnológico', link: '/es/arquitectura/stack-tecnologico' },
              { text: 'Arquitectura wZEC', link: '/es/arquitectura/wzec-arquitectura' },
              { text: 'Integración Attestation', link: '/es/arquitectura/integracion-attestation' }
            ]
          },
          {
            text: 'Guías',
            collapsed: true,
            items: [
              { text: 'Analytics Privado', link: '/es/guias/analytics-privado' },
              { text: 'Proof of Innocence', link: '/es/guias/proof-of-innocence' },
              { text: 'Integración SDK', link: '/es/guias/integracion-sdk' },
              { text: 'Referencia API', link: '/es/guias/referencia-api' },
              { text: 'Referencia API Completa', link: '/es/guias/referencia-api-completa' },
              { text: 'Endpoint Jobs', link: '/es/guias/api-endpoint-jobs' },
              { text: 'Slider Precios', link: '/es/guias/integracion-slider-precios' },
              { text: 'Ejemplos', link: '/es/guias/ejemplos' },
              { text: 'Despliegue', link: '/es/guias/despliegue' },
              { text: 'Configuración Prover', link: '/es/guias/configuracion-prover' },
              { text: 'Guía de la CLI Zyb', link: '/es/guias/guia-cli' },
              { text: 'Guía de Circuitos ZK', link: '/es/guias/circuitos-zk' },
              { text: 'Guía de Aplicación Web', link: '/es/guias/guia-webapp' },
              {
                text: 'Precios Dinámicos',
                collapsed: true,
                items: [
                    { text: 'Overview', link: '/es/guias/precios-dinamicos-overview' },
                    { text: 'Modelo', link: '/es/guias/modelo-precios-dinamicos' },
                    { text: 'Técnico', link: '/es/guias/precios-dinamicos-tecnico' },
                    { text: 'Uso', link: '/es/guias/precios-dinamicos-uso' }
                ]
              },
              {
                text: 'Integración wZEC',
                collapsed: true,
                items: [
                    { text: 'Guía Usuario', link: '/es/guias/wzec-guia-usuario' },
                    { text: 'Guía Desarrollador', link: '/es/guias/wzec-guia-desarrollador' },
                    { text: 'Referencia API', link: '/es/guias/wzec-referencia-api' },
                    { text: 'API wZEC', link: '/es/guias/wzec-api' },
                    { text: 'Guía Testing', link: '/es/guias/wzec-guia-testing' }
                ]
              },
              {
                text: 'Wallet de Referencia (Experimental)',
                collapsed: true,
                items: [
                  { text: 'Inicio Rápido', link: '/es/wallet-extension/README' },
                  { text: 'Desarrollo', link: '/es/wallet-extension/development' },
                  { text: 'Testing', link: '/es/wallet-extension/testing' },
                  { text: 'Testnets', link: '/es/wallet-extension/testnets' }
                ]
              }
            ]
          }
        ]
      }
    }
  }
})
