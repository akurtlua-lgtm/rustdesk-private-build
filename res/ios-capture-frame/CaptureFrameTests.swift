import CoreImage
import CoreVideo
import Foundation

@main
struct CaptureFrameTests {
    static func makeBuffer(_ format: OSType, width: Int, height: Int) -> CVPixelBuffer {
        var result: CVPixelBuffer?
        let status = CVPixelBufferCreate(
            kCFAllocatorDefault, width, height, format,
            [kCVPixelBufferIOSurfacePropertiesKey: [:]] as CFDictionary, &result
        )
        precondition(status == kCVReturnSuccess)
        return result!
    }

    static func main() throws {
        let context = CIContext(options: [.useSoftwareRenderer: true])
        let bgra = makeBuffer(kCVPixelFormatType_32BGRA, width: 3, height: 2)
        precondition(CVPixelBufferLockBaseAddress(bgra, []) == kCVReturnSuccess)
        let rowStride = CVPixelBufferGetBytesPerRow(bgra)
        precondition(rowStride > 12)
        let address = CVPixelBufferGetBaseAddress(bgra)!.assumingMemoryBound(to: UInt8.self)
        for y in 0..<2 {
            for x in 0..<3 {
                let pixel = address.advanced(by: y * rowStride + x * 4)
                pixel[0] = y == 0 ? 0 : 255; pixel[1] = 0
                pixel[2] = y == 0 ? 255 : 0; pixel[3] = 255
            }
        }
        precondition(CVPixelBufferUnlockBaseAddress(bgra, []) == kCVReturnSuccess)
        let rotated = try CaptureFrame.convert(bgra, orientation: .right, context: context)
        precondition(rotated.width == 2 && rotated.height == 3 && rotated.stride == 8)
        precondition(rotated.pixels.count == 24)
        for offset in stride(from: 0, to: rotated.pixels.count, by: 4) {
            let blue = (offset / 4) % 2 == 0
            precondition(rotated.pixels[offset] == (blue ? 255 : 0))
            precondition(rotated.pixels[offset + 1] == 0)
            precondition(rotated.pixels[offset + 2] == (blue ? 0 : 255))
            precondition(rotated.pixels[offset + 3] == 255)
        }
        print("Padded BGRA and rotated dimensions passed")

        let nv12 = makeBuffer(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange, width: 4, height: 4)
        precondition(CVPixelBufferLockBaseAddress(nv12, []) == kCVReturnSuccess)
        precondition(CVPixelBufferGetPlaneCount(nv12) == 2)
        for plane in 0..<2 {
            let bytes = CVPixelBufferGetBytesPerRowOfPlane(nv12, plane)
                * CVPixelBufferGetHeightOfPlane(nv12, plane)
            CVPixelBufferGetBaseAddressOfPlane(nv12, plane)!.initializeMemory(
                as: UInt8.self, repeating: plane == 0 ? 16 : 128, count: bytes
            )
        }
        precondition(CVPixelBufferUnlockBaseAddress(nv12, []) == kCVReturnSuccess)
        CVBufferSetAttachment(nv12, kCVImageBufferYCbCrMatrixKey,
                              kCVImageBufferYCbCrMatrix_ITU_R_709_2, .shouldPropagate)
        let black = try CaptureFrame.convert(nv12, orientation: .up, context: context)
        precondition(black.width == 4 && black.height == 4 && black.stride == 16)
        for offset in stride(from: 0, to: black.pixels.count, by: 4) {
            precondition(black.pixels[offset] < 10 && black.pixels[offset + 1] < 10)
            precondition(black.pixels[offset + 2] < 10 && black.pixels[offset + 3] > 240)
        }
        print("Video-range NV12 to packed BGRA passed")
        print("2 pixel buffer tests passed")
    }
}
