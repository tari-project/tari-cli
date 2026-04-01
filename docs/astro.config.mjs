// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  site: "https://tari-project.github.io",
  base: "/tari-cli",
  integrations: [
    starlight({
      favicon: "/favicon.png",
      title: "Tari CLI",
      description:
        "Develop, publish, and manage Tari Ootle templates with the Tari CLI.",
      head: [
        {
          tag: "meta",
          attrs: {
            property: "og:image",
            content:
              "https://tari-project.github.io/tari-cli/og-image.png",
          },
        },
        {
          tag: "meta",
          attrs: {
            property: "og:site_name",
            content: "Tari CLI",
          },
        },
        {
          tag: "meta",
          attrs: { name: "twitter:card", content: "summary_large_image" },
        },
        {
          tag: "meta",
          attrs: {
            name: "twitter:image",
            content:
              "https://tari-project.github.io/tari-cli/og-image.png",
          },
        },
      ],
      customCss: [
        "./src/styles/global.scss",
        "./src/styles/custom.scss",
        "./src/fonts/font-face.css",
      ],
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/tari-project/tari-cli",
        },
      ],
      sidebar: [
        {
          label: "Getting Started",
          items: [
            { label: "Installation", link: "/guides/installation/" },
            { label: "Quick Start", link: "/guides/quick-start/" },
          ],
        },
        {
          label: "Commands",
          items: [
            { label: "create", link: "/reference/create/" },
            { label: "add", link: "/reference/add/" },
            { label: "publish", link: "/reference/publish/" },
            { label: "template init", link: "/reference/template-init/" },
            {
              label: "template publish",
              link: "/reference/template-publish/",
            },
            {
              label: "template inspect",
              link: "/reference/template-inspect/",
            },
          ],
        },
        {
          label: "Guides",
          items: [
            {
              label: "Template Metadata",
              link: "/guides/template-metadata/",
            },
          ],
        },
      ],
      components: {
        Pagination: "./src/components/Pagination.astro",
      },
    }),
  ],
});
