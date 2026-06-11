package api

import (
	"fmt"
	"net/http"
	"strconv"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/thearchitectit/guardrail-mcp/internal/vision"
)

// VisionServer exposes vision pipeline endpoints over HTTP.
type VisionServer struct {
	engine  *vision.ReviewEngine
	watcher *vision.CaptureWatcher
}

// NewVisionServer creates the HTTP API for vision operations.
func NewVisionServer(engine *vision.ReviewEngine, watcher *vision.CaptureWatcher) *VisionServer {
	return &VisionServer{engine: engine, watcher: watcher}
}

// RegisterRoutes mounts vision routes on the given Echo group.
func (vs *VisionServer) RegisterRoutes(g *echo.Group) {
	g.POST("/review", vs.handleReview)
	g.GET("/review/:id", vs.handleGetReview)
	g.POST("/review/:id/iterate", vs.handleIterate)
	g.GET("/reviews", vs.handleListReviews)
	g.GET("/events", vs.handleEvents)
	g.POST("/capture/trigger", vs.handleCaptureTrigger)
	g.GET("/health", vs.handleHealth)
}

func (vs *VisionServer) handleReview(c echo.Context) error {
	var req struct {
		ImagePath string `json:"image_path"`
		ImageB64  string `json:"image_b64"`
	}
	if err := c.Bind(&req); err != nil {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": err.Error()})
	}

	var path string
	if req.ImagePath != "" {
		path = req.ImagePath
	} else if req.ImageB64 != "" {
		// Decode base64 to temp file
		// Simplified: require path for now
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "image_path is required (base64 not yet supported)"})
	} else {
		return c.JSON(http.StatusBadRequest, map[string]string{"error": "image_path or image_b64 required"})
	}

	report, err := vs.engine.Run(c.Request().Context(), path)
	if err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}
	return c.JSON(http.StatusOK, report)
}

func (vs *VisionServer) handleGetReview(c echo.Context) error {
	id := c.Param("id")
	report, err := vs.engine.GetReport(id)
	if err != nil {
		return c.JSON(http.StatusNotFound, map[string]string{"error": err.Error()})
	}
	return c.JSON(http.StatusOK, report)
}

func (vs *VisionServer) handleIterate(c echo.Context) error {
	id := c.Param("id")
	report, err := vs.engine.Iterate(c.Request().Context(), id)
	if err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}
	return c.JSON(http.StatusOK, report)
}

func (vs *VisionServer) handleListReviews(c echo.Context) error {
	limit := 50
	if l := c.QueryParam("limit"); l != "" {
		if parsed, err := strconv.Atoi(l); err == nil && parsed > 0 {
			limit = parsed
		}
	}
	reviews, err := vs.engine.ListReviews(limit)
	if err != nil {
		return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
	}
	return c.JSON(http.StatusOK, reviews)
}

func (vs *VisionServer) handleEvents(c echo.Context) error {
	// SSE endpoint for capture and review events
	c.Response().Header().Set("Content-Type", "text/event-stream")
	c.Response().Header().Set("Cache-Control", "no-cache")
	c.Response().Header().Set("Connection", "keep-alive")
	c.Response().WriteHeader(http.StatusOK)

	// Simplified: send a heartbeat every 5s
	ticker := time.NewTicker(5 * time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-c.Request().Context().Done():
			return nil
		case <-ticker.C:
			fmt.Fprintf(c.Response(), ": ping\n\n")
			c.Response().Flush()
		}
	}
}

func (vs *VisionServer) handleCaptureTrigger(c echo.Context) error {
	msg := vs.watcher.TriggerCapture()
	return c.JSON(http.StatusOK, map[string]string{"message": msg})
}

func (vs *VisionServer) handleHealth(c echo.Context) error {
	status, err := vs.engine.HealthCheck(c.Request().Context())
	if err != nil {
		return c.JSON(http.StatusServiceUnavailable, status)
	}
	return c.JSON(http.StatusOK, status)
}
