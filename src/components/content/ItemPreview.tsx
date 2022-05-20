import { ExclamationOutlined } from "@ant-design/icons";
import { Avatar } from "antd";
import { useMemo } from "react";
import { useImageFile } from "../../hooks/useImage";

interface SubImage {
    path: string,
    index: number
}

interface ItemPreviewProps {
    inventoryGraphics: {
        small: SubImage,
        big: SubImage,
    }
}

function graphicTypeToItemPrefix(graphicType: number): string {
    switch (graphicType)
    {
        case 0:
            return "GUN";
        case 1:
            return "P1ITEM";
        case 2:
            return "P2ITEM";
    }
    return "P3ITEM"
}

const crispEdgesStyle = {imageRendering: "crisp-edges" as const};

export function ItemPreview({ inventoryGraphics: { big } }: ItemPreviewProps) {
    const { data: image, error } = useImageFile(big.path);
    const additionalAvatarProps = useMemo(() => {
        if (error) {
            console.error(error);
            return { icon: <ExclamationOutlined /> }
        }
        if (!image) {
            return {};
        }
        return { src: image };
    }, [error, image]);

    if (error) {
        return <span>Error loading image: {error.toString()}</span>
    }

    return <Avatar shape="square" {...additionalAvatarProps} style={crispEdgesStyle} />;
}