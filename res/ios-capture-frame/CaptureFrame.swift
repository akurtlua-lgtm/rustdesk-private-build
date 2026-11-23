import CoreImage
import CoreVideo
import Foundation
import ImageIO

struct CaptureFrame: Sendable {
    let width: Int
    let height: Int
    let stride: Int
    let pixels: Data

    enum ConversionError: Error {
        case unsupportedFormat
        case invalidDimensions
        case renderFailed
    }

    static func convert(
        _ buffer: CVPixelBuffer,
        orientation: CGImagePropertyOrientation,
        context: CIContext
    ) throws -> CaptureFrame {
        switch CVPixelBufferGetPixelFormatType(buffer) {
        case kCVPixelFormatType_32BGRA,
             kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange,
             kCVPixelFormatType_420YpCbCr8BiPlanarFullRange:
            break
        default:
            throw ConversionError.unsupportedFormat
        }
        let image = CIImage(cvPixelBuffer: buffer).oriented(orientation)
        let bounds = image.extent.integral
        guard !bounds.isNull, !bounds.isInfinite,
              bounds.width.isFinite, bounds.height.isFinite,
              bounds.width > 0, bounds.height > 0,
              bounds.width <= 8192, bounds.height <= 8192 else {
            throw ConversionError.invalidDimensions
        }
        let width = Int(bounds.width)
        let height = Int(bounds.height)
        let stride = width * 4
        guard stride * height <= 32 * 1024 * 1024 else {
            throw ConversionError.invalidDimensions
        }
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        guard let rendered = context.createCGImage(
            image, from: bounds, format: .BGRA8, colorSpace: colorSpace
        ) else {
            throw ConversionError.renderFailed
        }
        var pixels = Data(count: stride * height)
        let drawn = pixels.withUnsafeMutableBytes { bytes -> Bool in
            guard let address = bytes.baseAddress,
                  let bitmap = CGContext(
                    data: address, width: width, height: height,
                    bitsPerComponent: 8, bytesPerRow: stride, space: colorSpace,
                    bitmapInfo: CGBitmapInfo.byteOrder32Little.rawValue
                        | CGImageAlphaInfo.premultipliedFirst.rawValue
                  ) else {
                return false
            }
            bitmap.draw(rendered, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard drawn else {
            throw ConversionError.renderFailed
        }
        return CaptureFrame(width: width, height: height, stride: stride, pixels: pixels)
    }
}
