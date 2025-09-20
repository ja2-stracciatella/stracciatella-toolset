const rendererModuleConfig = {
    rules: [
        {
            test: /\.s?css$/,
            use: [
                'style-loader',
                {
                    loader: 'css-loader',
                    options: {
                        modules: true,
                        sourceMap: true,
                        importLoaders: 1,
                    },
                },
                'sass-loader',
            ],
            include: /\.module\.s?(c|a)ss$/,
        },
        {
            test: /\.s?css$/,
            use: ['style-loader', 'css-loader', 'sass-loader'],
            exclude: /\.module\.s?(c|a)ss$/,
        },
        // Fonts
        {
            test: /\.(woff|woff2|eot|ttf|otf)$/i,
            type: 'asset/resource',
        },
        // Images
        {
            test: /\.(png|jpg|jpeg|gif)$/i,
            type: 'asset/resource',
        },
        // SVG
        {
            test: /\.svg$/,
            use: [
                {
                    loader: '@svgr/webpack',
                    options: {
                        prettier: false,
                        svgo: false,
                        svgoConfig: {
                            plugins: [{ removeViewBox: false }],
                        },
                        titleProp: true,
                        ref: true,
                    },
                },
            ],
        },
    ],
};
export default rendererModuleConfig;
