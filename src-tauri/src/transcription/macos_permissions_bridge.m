#import <AppKit/AppKit.h>
#import <CoreAudio/CoreAudioTypes.h>
#import <CoreGraphics/CoreGraphics.h>
#import <CoreMedia/CoreMedia.h>
#import <Foundation/Foundation.h>
#import <ScreenCaptureKit/ScreenCaptureKit.h>

typedef void (*OpenNoteAudioCallback)(const float *samples, size_t count, void *context);

static void OpenNoteSetError(char **error, NSString *message) {
    if (error != NULL) {
        *error = strdup(message.UTF8String);
    }
}

@interface OpenNoteSystemAudioCapture : NSObject <SCStreamOutput>
@property(nonatomic, strong) SCStream *stream;
@property(nonatomic) dispatch_queue_t queue;
@property(nonatomic) OpenNoteAudioCallback callback;
@property(nonatomic) void *context;
@end

@implementation OpenNoteSystemAudioCapture
- (void)stream:(SCStream *)stream
    didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
                  ofType:(SCStreamOutputType)type {
    if (type != SCStreamOutputTypeAudio || self.callback == NULL) {
        return;
    }

    CMFormatDescriptionRef format = CMSampleBufferGetFormatDescription(sampleBuffer);
    const AudioStreamBasicDescription *description =
        CMAudioFormatDescriptionGetStreamBasicDescription(format);
    if (description == NULL ||
        description->mFormatID != kAudioFormatLinearPCM ||
        (description->mFormatFlags & kAudioFormatFlagIsFloat) == 0 ||
        description->mChannelsPerFrame != 1) {
        return;
    }

    size_t bufferListSize = 0;
    CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
        sampleBuffer, &bufferListSize, NULL, 0, NULL, NULL,
        kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment, NULL);
    AudioBufferList *bufferList = malloc(bufferListSize);
    if (bufferList == NULL) {
        return;
    }

    CMBlockBufferRef blockBuffer = NULL;
    OSStatus status = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
        sampleBuffer, NULL, bufferList, bufferListSize, NULL, NULL,
        kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment, &blockBuffer);
    if (status == noErr && bufferList->mNumberBuffers > 0) {
        AudioBuffer buffer = bufferList->mBuffers[0];
        size_t count = buffer.mDataByteSize / sizeof(float);
        self.callback((const float *)buffer.mData, count, self.context);
    }

    if (blockBuffer != NULL) {
        CFRelease(blockBuffer);
    }
    free(bufferList);
}
@end

bool opennote_check_screen_capture_permission(void) {
    return CGPreflightScreenCaptureAccess();
}

bool opennote_open_screen_capture_settings(void) {
    NSURL *url = [NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"];
    return [[NSWorkspace sharedWorkspace] openURL:url];
}

bool opennote_is_macos_13_or_newer(void) {
    if (@available(macOS 13.0, *)) {
        return true;
    }
    return false;
}

const char *opennote_screen_capture_permission_error(void) {
    return "Screen Recording permission is required to record computer audio. Enable it in System Settings > Privacy & Security > Screen Recording, then restart OpenNote.";
}

void *opennote_start_system_audio_capture(
    OpenNoteAudioCallback callback,
    void *context,
    char **error
) {
    if (@available(macOS 13.0, *)) {
        dispatch_semaphore_t contentSemaphore = dispatch_semaphore_create(0);
        __block SCShareableContent *shareableContent = nil;
        __block NSError *contentError = nil;
        [SCShareableContent
            getShareableContentExcludingDesktopWindows:YES
                                   onScreenWindowsOnly:YES
                                    completionHandler:^(SCShareableContent *content, NSError *fetchError) {
            shareableContent = content;
            contentError = fetchError;
            dispatch_semaphore_signal(contentSemaphore);
        }];

        dispatch_semaphore_wait(contentSemaphore, DISPATCH_TIME_FOREVER);
        if (contentError != nil) {
            OpenNoteSetError(error, contentError.localizedDescription);
            return NULL;
        }
        SCDisplay *display = shareableContent.displays.firstObject;
        if (display == nil) {
            OpenNoteSetError(error, @"No display is available for computer audio capture.");
            return NULL;
        }

        SCContentFilter *filter =
            [[SCContentFilter alloc] initWithDisplay:display excludingWindows:@[]];
        SCStreamConfiguration *configuration = [[SCStreamConfiguration alloc] init];
        configuration.capturesAudio = YES;
        configuration.excludesCurrentProcessAudio = YES;
        configuration.sampleRate = 16000;
        configuration.channelCount = 1;

        OpenNoteSystemAudioCapture *capture = [[OpenNoteSystemAudioCapture alloc] init];
        capture.callback = callback;
        capture.context = context;
        capture.queue = dispatch_queue_create("dev.caleblee.opennote.system-audio", DISPATCH_QUEUE_SERIAL);
        capture.stream = [[SCStream alloc] initWithFilter:filter configuration:configuration delegate:nil];

        NSError *addOutputError = nil;
        if (![capture.stream addStreamOutput:capture
                                        type:SCStreamOutputTypeAudio
                          sampleHandlerQueue:capture.queue
                                       error:&addOutputError]) {
            OpenNoteSetError(error, addOutputError.localizedDescription);
            return NULL;
        }

        dispatch_semaphore_t startSemaphore = dispatch_semaphore_create(0);
        __block NSError *startError = nil;
        [capture.stream startCaptureWithCompletionHandler:^(NSError *captureError) {
            startError = captureError;
            dispatch_semaphore_signal(startSemaphore);
        }];
        dispatch_semaphore_wait(startSemaphore, DISPATCH_TIME_FOREVER);
        if (startError != nil) {
            OpenNoteSetError(error, startError.localizedDescription);
            return NULL;
        }
        return (__bridge_retained void *)capture;
    }

    OpenNoteSetError(error, @"Computer audio recording requires macOS 13 or newer.");
    return NULL;
}

bool opennote_stop_system_audio_capture(void *handle, char **error) {
    if (handle == NULL) {
        return true;
    }
    OpenNoteSystemAudioCapture *capture =
        (__bridge_transfer OpenNoteSystemAudioCapture *)handle;
    dispatch_semaphore_t stopSemaphore = dispatch_semaphore_create(0);
    __block NSError *stopError = nil;
    [capture.stream stopCaptureWithCompletionHandler:^(NSError *captureError) {
        stopError = captureError;
        dispatch_semaphore_signal(stopSemaphore);
    }];
    dispatch_semaphore_wait(stopSemaphore, DISPATCH_TIME_FOREVER);
    if (stopError != nil) {
        OpenNoteSetError(error, stopError.localizedDescription);
        return false;
    }
    return true;
}

void opennote_free_error(char *error) {
    free(error);
}
